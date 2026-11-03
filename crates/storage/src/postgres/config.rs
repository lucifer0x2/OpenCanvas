use super::*;

/// Blocking-operation limits. The connection string is deliberately not Debug.
#[derive(Clone)]
pub struct PostgresConfig {
    pub(crate) url: String,
    pub connect_timeout: Duration,
    pub statement_timeout: Duration,
    pub lock_timeout: Duration,
    pub operation_timeout: Duration,
    pub pool_size: usize,
}

impl PostgresConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connect_timeout: Duration::from_secs(3),
            statement_timeout: Duration::from_secs(15),
            lock_timeout: Duration::from_secs(3),
            operation_timeout: Duration::from_secs(30),
            pool_size: 4,
        }
    }

    pub(super) fn driver(&self) -> Result<::postgres::Config> {
        if self.pool_size == 0
            || self.pool_size > 32
            || [
                self.connect_timeout,
                self.statement_timeout,
                self.lock_timeout,
                self.operation_timeout,
            ]
            .iter()
            .any(Duration::is_zero)
        {
            return Err(crate::StorageError::validation(
                "postgres",
                "Invalid database limits",
            ));
        }
        let mut config: ::postgres::Config = self.url.parse().map_err(|_| {
            crate::StorageError::validation("postgres", "Invalid database configuration")
        })?;
        config.connect_timeout(self.connect_timeout)
            .tcp_user_timeout(self.operation_timeout)
            .keepalives(true)
            .keepalives_idle(Duration::from_secs(3))
            .keepalives_interval(Duration::from_secs(1))
            .keepalives_retries(3)
            .application_name("opencanvas-runtime")
            .options(&format!(
                "-c search_path=opencanvas_runtime_v1,pg_catalog -c statement_timeout={} -c lock_timeout={} -c idle_in_transaction_session_timeout={} -c synchronous_commit=on",
                milliseconds(self.statement_timeout), milliseconds(self.lock_timeout), milliseconds(self.operation_timeout),
            ));
        Ok(config)
    }
}
