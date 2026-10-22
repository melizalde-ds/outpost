use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Unique constraint violated: {0}")]
    UniqueViolation(String),
    #[error("Database unavailable: {0}")]
    Unavailable(#[source] sqlx::Error),
    #[error("Database error: {0}")]
    Database(#[source] sqlx::Error),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match &err {
            sqlx::Error::Io(_) | sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed => {
                Self::Unavailable(err)
            }
            sqlx::Error::Database(db) => match db.code().as_deref() {
                Some("23505") => {
                    Self::UniqueViolation(db.constraint().unwrap_or("unknown").to_string())
                }
                Some(code) if code.starts_with("08") || code.starts_with("57P") => {
                    Self::Unavailable(err)
                }
                _ => Self::Database(err),
            },
            _ => Self::Database(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, error::Error as StdError, fmt, io};

    use rstest::rstest;
    use sqlx::error::{DatabaseError, ErrorKind};

    use super::*;

    #[derive(Debug)]
    struct PgError(&'static str);

    impl fmt::Display for PgError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "postgres error {}", self.0)
        }
    }

    impl StdError for PgError {}

    impl DatabaseError for PgError {
        fn message(&self) -> &str {
            "postgres error"
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            Some(Cow::Borrowed(self.0))
        }

        fn as_error(&self) -> &(dyn StdError + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn StdError + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn StdError + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    fn postgres(code: &'static str) -> sqlx::Error {
        sqlx::Error::Database(Box::new(PgError(code)))
    }

    #[rstest]
    #[case::pool_timed_out(sqlx::Error::PoolTimedOut)]
    #[case::pool_closed(sqlx::Error::PoolClosed)]
    #[case::connection_refused(sqlx::Error::Io(io::ErrorKind::ConnectionRefused.into()))]
    #[case::connection_failure(postgres("08006"))]
    #[case::server_shutting_down(postgres("57P01"))]
    #[case::server_starting_up(postgres("57P03"))]
    fn from_database_out_of_reach_returns_unavailable(#[case] err: sqlx::Error) {
        // Act
        let result = RepositoryError::from(err);

        // Assert
        assert!(matches!(result, RepositoryError::Unavailable(_)));
    }

    #[rstest]
    #[case::row_not_found(sqlx::Error::RowNotFound)]
    #[case::syntax_error(postgres("42601"))]
    #[case::query_cancelled(postgres("57014"))]
    fn from_other_failure_returns_database(#[case] err: sqlx::Error) {
        // Act
        let result = RepositoryError::from(err);

        // Assert
        assert!(matches!(result, RepositoryError::Database(_)));
    }

    #[test]
    fn from_unique_violation_returns_unique_violation() {
        // Act
        let result = RepositoryError::from(postgres("23505"));

        // Assert
        assert!(matches!(result, RepositoryError::UniqueViolation(_)));
    }
}
