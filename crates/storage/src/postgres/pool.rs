//! Dedicated reusable sync clients, with finite checkout and response waits.
//! Each worker's prepared statements are dropped together with its client.
use super::*;
use ::postgres::{Config, NoTls};
use std::sync::{mpsc, Condvar, Mutex};
use std::time::Instant;

type Job = Box<dyn FnOnce(&mut Option<Session>, &Config) + Send>;
type Idle = Arc<(Mutex<Vec<usize>>, Condvar)>;

pub(super) struct Pool {
    workers: Vec<mpsc::SyncSender<Job>>,
    idle: Idle,
    timeout: Duration,
}

impl Pool {
    pub fn new(config: PostgresConfig) -> Result<Self> {
        let driver = Arc::new(config.driver()?);
        let idle: Idle = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let mut workers = Vec::new();
        for index in 0..config.pool_size {
            let (sender, receiver) = mpsc::sync_channel::<Job>(1);
            let idle = idle.clone();
            let driver = driver.clone();
            std::thread::Builder::new()
                .name(format!("opencanvas-pg-{index}"))
                .spawn(move || {
                    let mut session = None;
                    while let Ok(job) = receiver.recv() {
                        job(&mut session, &driver);
                        if session
                            .as_ref()
                            .is_some_and(|open: &Session| open.client.is_closed())
                        {
                            session = None;
                        }
                        let (lock, condition) = &*idle;
                        if let Ok(mut available) = lock.lock() {
                            available.push(index);
                            condition.notify_one();
                        }
                    }
                })
                .map_err(|_| unavailable())?;
            workers.push(sender);
        }
        idle.0
            .lock()
            .map_err(|_| unavailable())?
            .extend(0..config.pool_size);
        Ok(Self {
            workers,
            idle,
            timeout: config.operation_timeout,
        })
    }

    pub fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut Session) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let deadline = Instant::now() + self.timeout;
        let (lock, condition) = &*self.idle;
        let mut available = lock.lock().map_err(|_| unavailable())?;
        let index = loop {
            if let Some(index) = available.pop() {
                break index;
            }
            let wait = deadline.saturating_duration_since(Instant::now());
            if wait.is_zero() {
                return Err(unavailable());
            }
            let (next, timed) = condition
                .wait_timeout(available, wait)
                .map_err(|_| unavailable())?;
            available = next;
            if timed.timed_out() {
                return Err(unavailable());
            }
        };
        drop(available);
        let (sender, receiver) = mpsc::channel();
        let job = Box::new(move |session: &mut Option<Session>, driver: &Config| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if session.is_none() {
                    *session = Some(Session::new(driver.connect(NoTls).map_err(sql)?));
                }
                operation(session.as_mut().expect("connected session"))
            }))
            .unwrap_or_else(|_| {
                *session = None;
                Err(unavailable())
            });
            let broken = matches!(&result, Err(error) if error.code == "STORAGE_UNAVAILABLE");
            let _ = sender.send(result);
            // Reply first; a possibly broken socket is then closed off the caller's path
            // and the next job on this worker reconnects.
            if broken {
                *session = None;
            }
        });
        self.workers[index].send(job).map_err(|_| unavailable())?;
        receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(|_| unavailable())?
    }
}
