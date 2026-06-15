//! Error type for the state store.

use thiserror::Error;

/// Convenience result alias for state operations.
pub type Result<T> = std::result::Result<T, StateError>;

/// Errors that can arise while reading or writing the durable store.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StateError {
    /// Opening or creating the database failed.
    #[error("database error: {0}")]
    Database(#[from] redb::DatabaseError),

    /// Beginning a transaction failed.
    #[error("transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),

    /// Opening a table failed.
    #[error("table error: {0}")]
    Table(#[from] redb::TableError),

    /// A read or write against storage failed.
    #[error("storage error: {0}")]
    Storage(#[from] redb::StorageError),

    /// Committing a transaction failed.
    #[error("commit error: {0}")]
    Commit(#[from] redb::CommitError),

    /// Serializing or deserializing a stored value failed.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
