//! Per-connection prepared statements keyed by static SQL text.
//! A statement is valid only on the connection that prepared it, so the cache
//! lives and dies with the worker's client.
use super::*;
use ::postgres::{
    error::SqlState, types::ToSql, Client, GenericClient, IsolationLevel, Row, Statement,
    Transaction,
};
use std::collections::HashMap;

type Params<'p> = &'p [&'p (dyn ToSql + Sync)];

/// One worker's connection and the statements prepared on it.
pub(super) struct Session {
    pub client: Client,
    pub statements: Statements,
}

impl Session {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            statements: Statements::default(),
        }
    }

    /// Statements outside an explicit transaction.
    pub fn db(&mut self) -> Db<'_, Client> {
        Db::new(&mut self.client, &mut self.statements)
    }

    /// A repeatable-read, read-only transaction and this connection's statements.
    pub fn read_only(&mut self) -> Result<(Transaction<'_>, &mut Statements)> {
        let tx = self
            .client
            .build_transaction()
            .isolation_level(IsolationLevel::RepeatableRead)
            .read_only(true)
            .start()
            .map_err(sql)?;
        Ok((tx, &mut self.statements))
    }
}

#[derive(Default)]
pub(super) struct Statements(HashMap<&'static str, Statement>);

/// A client or transaction paired with its connection's statement cache.
/// Statements prepared inside a transaction remain valid for the connection.
pub(super) struct Db<'a, C> {
    client: &'a mut C,
    statements: &'a mut Statements,
}

impl<'a, C: GenericClient> Db<'a, C> {
    pub fn new(client: &'a mut C, statements: &'a mut Statements) -> Self {
        Self { client, statements }
    }

    pub fn query(&mut self, text: &'static str, params: Params<'_>) -> Result<Vec<Row>> {
        let statement = self.prepared(text)?;
        let result = self.client.query(&statement, params);
        self.checked(result)
    }

    pub fn query_opt(&mut self, text: &'static str, params: Params<'_>) -> Result<Option<Row>> {
        let statement = self.prepared(text)?;
        let result = self.client.query_opt(&statement, params);
        self.checked(result)
    }

    pub fn execute(&mut self, text: &'static str, params: Params<'_>) -> Result<u64> {
        let statement = self.prepared(text)?;
        let result = self.client.execute(&statement, params);
        self.checked(result)
    }

    fn prepared(&mut self, text: &'static str) -> Result<Statement> {
        if let Some(statement) = self.statements.0.get(text) {
            return Ok(statement.clone());
        }
        let statement = self.client.prepare(text).map_err(sql)?;
        self.statements.0.insert(text, statement.clone());
        Ok(statement)
    }

    /// A schema change can invalidate a cached plan's result type, and a lost
    /// server-side statement is unknown. Forget every statement so the next call
    /// re-prepares; the failed call itself is never retried.
    fn checked<T>(&mut self, result: std::result::Result<T, ::postgres::Error>) -> Result<T> {
        result.map_err(|error| {
            if stale(error.code()) {
                self.statements.0.clear();
            }
            sql(error)
        })
    }
}

fn stale(code: Option<&SqlState>) -> bool {
    code.is_some_and(|code| {
        [
            SqlState::FEATURE_NOT_SUPPORTED,
            SqlState::INVALID_SQL_STATEMENT_NAME,
        ]
        .contains(code)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plan_invalidation_and_unknown_statements_clear_the_cache() {
        assert!(stale(Some(&SqlState::FEATURE_NOT_SUPPORTED)));
        assert!(stale(Some(&SqlState::INVALID_SQL_STATEMENT_NAME)));
        assert!(!stale(Some(&SqlState::UNIQUE_VIOLATION)));
        assert!(!stale(Some(&SqlState::LOCK_NOT_AVAILABLE)));
        assert!(!stale(None));
    }
}
