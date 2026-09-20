use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("unique constraint violated: {0}")]
    UniqueViolation(String),
    #[error("database error: {0}")]
    Database(#[source] sqlx::Error),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref db) = err
            && db.code().as_deref() == Some("23505")
        {
            return Self::UniqueViolation(db.constraint().unwrap_or("unknown").to_string());
        }
        Self::Database(err)
    }
}
