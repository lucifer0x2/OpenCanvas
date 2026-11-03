//! Optional PostgreSQL pool and timeout limits read from the environment.
//! Error messages name the variable only; they never echo DATABASE_URL.

use opencanvas_storage::PostgresConfig;
use std::{ffi::OsString, time::Duration};

pub const POOL_SIZE: &str = "DATABASE_POOL_SIZE";
pub const CONNECT_TIMEOUT: &str = "DATABASE_CONNECT_TIMEOUT_MS";
pub const STATEMENT_TIMEOUT: &str = "DATABASE_STATEMENT_TIMEOUT_MS";
pub const LOCK_TIMEOUT: &str = "DATABASE_LOCK_TIMEOUT_MS";
pub const OPERATION_TIMEOUT: &str = "DATABASE_OPERATION_TIMEOUT_MS";

/// Largest pool the storage adapter accepts.
const MAX_POOL_SIZE: u64 = 32;
/// PostgreSQL timeout settings are signed 32-bit milliseconds.
const MAX_TIMEOUT_MS: u64 = i32::MAX as u64;

/// Builds the backend configuration, overriding defaults with any variable `lookup` returns.
pub fn config(
    url: String,
    lookup: impl Fn(&str) -> Option<OsString>,
) -> Result<PostgresConfig, String> {
    let mut config = PostgresConfig::new(url);
    let read = |name: &str, max: u64, unit: &str| -> Result<Option<u64>, String> {
        let Some(value) = lookup(name) else {
            return Ok(None);
        };
        value
            .to_str()
            .and_then(|text| text.parse::<u64>().ok())
            .filter(|number| (1..=max).contains(number))
            .map(Some)
            .ok_or_else(|| format!("{name} must be an integer from 1 to {max}{unit}"))
    };
    if let Some(size) = read(POOL_SIZE, MAX_POOL_SIZE, "")? {
        config.pool_size = size as usize;
    }
    for (name, field) in [
        (CONNECT_TIMEOUT, &mut config.connect_timeout),
        (STATEMENT_TIMEOUT, &mut config.statement_timeout),
        (LOCK_TIMEOUT, &mut config.lock_timeout),
        (OPERATION_TIMEOUT, &mut config.operation_timeout),
    ] {
        if let Some(ms) = read(name, MAX_TIMEOUT_MS, " milliseconds")? {
            *field = Duration::from_millis(ms);
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const URL: &str = "postgresql://user:secret-must-not-appear@db/opencanvas";

    fn with(vars: &[(&str, &str)]) -> Result<PostgresConfig, String> {
        let vars: HashMap<String, OsString> = vars
            .iter()
            .map(|(name, value)| (name.to_string(), OsString::from(value)))
            .collect();
        config(URL.into(), |name| vars.get(name).cloned())
    }

    #[test]
    fn unset_variables_keep_storage_defaults() {
        let config = with(&[]).unwrap();
        let defaults = PostgresConfig::new(URL);
        assert_eq!(config.pool_size, defaults.pool_size);
        assert_eq!(config.connect_timeout, defaults.connect_timeout);
        assert_eq!(config.statement_timeout, defaults.statement_timeout);
        assert_eq!(config.lock_timeout, defaults.lock_timeout);
        assert_eq!(config.operation_timeout, defaults.operation_timeout);
    }

    #[test]
    fn valid_overrides_apply_to_each_limit() {
        let config = with(&[
            (POOL_SIZE, "32"),
            (CONNECT_TIMEOUT, "250"),
            (STATEMENT_TIMEOUT, "60000"),
            (LOCK_TIMEOUT, "1"),
            (OPERATION_TIMEOUT, "2147483647"),
        ])
        .unwrap();
        assert_eq!(config.pool_size, 32);
        assert_eq!(config.connect_timeout, Duration::from_millis(250));
        assert_eq!(config.statement_timeout, Duration::from_secs(60));
        assert_eq!(config.lock_timeout, Duration::from_millis(1));
        assert_eq!(
            config.operation_timeout,
            Duration::from_millis(2_147_483_647)
        );
        assert_eq!(with(&[(POOL_SIZE, "1")]).unwrap().pool_size, 1);
    }

    #[test]
    fn invalid_values_name_the_variable_without_the_url() {
        let cases = [
            (POOL_SIZE, "0"),
            (POOL_SIZE, "33"),
            (POOL_SIZE, "-1"),
            (POOL_SIZE, "4.5"),
            (POOL_SIZE, ""),
            (CONNECT_TIMEOUT, "0"),
            (STATEMENT_TIMEOUT, "fast"),
            (LOCK_TIMEOUT, " 3000"),
            (OPERATION_TIMEOUT, "2147483648"),
        ];
        for (name, value) in cases {
            let error = match with(&[(name, value)]) {
                Ok(_) => panic!("{name}={value:?} accepted"),
                Err(error) => error,
            };
            assert!(error.starts_with(name), "{error}");
            assert!(!error.contains("secret-must-not-appear"));
        }
    }
}
