//! The `redb`-backed state store.

use crate::error::Result;
use crate::token::TokenRecord;
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;
use steward_core::TokenId;

const TOKENS: TableDefinition<&str, &str> = TableDefinition::new("tokens");
const REVOCATIONS: TableDefinition<&str, i64> = TableDefinition::new("revocations");
const CHECKPOINTS: TableDefinition<&str, &str> = TableDefinition::new("checkpoints");
const META: TableDefinition<&str, &str> = TableDefinition::new("meta");

const AUDIT_HEAD_KEY: &str = "audit_head";

/// The durable state store. Cheap to share behind a reference; all methods take
/// `&self` and open a fresh transaction.
pub struct StateStore {
    db: Database,
}

impl StateStore {
    /// Opens (creating if needed) the store at `path`.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) if the database cannot be
    /// opened or its tables cannot be initialized.
    pub fn open(path: &Path) -> Result<Self> {
        let db = Database::create(path)?;
        Self::from_db(db)
    }

    /// Opens an in-memory store, primarily for tests.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) if the database cannot be
    /// created or its tables cannot be initialized.
    pub fn open_in_memory() -> Result<Self> {
        let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
        Self::from_db(db)
    }

    fn from_db(db: Database) -> Result<Self> {
        // Create all tables up front so reads never hit a missing table.
        let txn = db.begin_write()?;
        {
            txn.open_table(TOKENS)?;
            txn.open_table(REVOCATIONS)?;
            txn.open_table(CHECKPOINTS)?;
            txn.open_table(META)?;
        }
        txn.commit()?;
        Ok(Self { db })
    }

    /// Inserts or replaces a token record.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on serialization or storage failure.
    pub fn put_token(&self, record: &TokenRecord) -> Result<()> {
        let json = serde_json::to_string(record)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(TOKENS)?;
            table.insert(record.id.as_str(), json.as_str())?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Fetches a token record by id, if present.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage or deserialization failure.
    pub fn get_token(&self, id: &TokenId) -> Result<Option<TokenRecord>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(TOKENS)?;
        match table.get(id.as_str())? {
            Some(value) => Ok(Some(serde_json::from_str(value.value())?)),
            None => Ok(None),
        }
    }

    /// Lists all token records.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage or deserialization failure.
    pub fn list_tokens(&self) -> Result<Vec<TokenRecord>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(TOKENS)?;
        let mut records = Vec::new();
        for entry in table.iter()? {
            let (_key, value) = entry?;
            records.push(serde_json::from_str(value.value())?);
        }
        Ok(records)
    }

    /// Marks a token as revoked at the given Unix time.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn revoke(&self, id: &TokenId, at_unix: i64) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(REVOCATIONS)?;
            table.insert(id.as_str(), at_unix)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Returns `true` if the token has been revoked.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn is_revoked(&self, id: &TokenId) -> Result<bool> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(REVOCATIONS)?;
        Ok(table.get(id.as_str())?.is_some())
    }

    /// Stores a checkpoint's recovery blob (opaque JSON) under `id`.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn put_checkpoint(&self, id: &str, recovery_json: &str) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(CHECKPOINTS)?;
            table.insert(id, recovery_json)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Fetches a checkpoint's recovery blob, if present.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn get_checkpoint(&self, id: &str) -> Result<Option<String>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(CHECKPOINTS)?;
        Ok(table.get(id)?.map(|v| v.value().to_owned()))
    }

    /// Deletes a checkpoint. Removing a missing checkpoint is not an error.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn delete_checkpoint(&self, id: &str) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(CHECKPOINTS)?;
            table.remove(id)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Persists the audit log head hash.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn set_audit_head(&self, head: &str) -> Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(META)?;
            table.insert(AUDIT_HEAD_KEY, head)?;
        }
        txn.commit()?;
        Ok(())
    }

    /// Returns the persisted audit head hash, or an empty string if unset.
    ///
    /// # Errors
    /// Returns a [`StateError`](crate::StateError) on storage failure.
    pub fn audit_head(&self) -> Result<String> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(META)?;
        Ok(table
            .get(AUDIT_HEAD_KEY)?
            .map_or_else(String::new, |v| v.value().to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use steward_core::{Capability, CapabilitySet, OperationId, RiskLevel, ServerId};

    fn record(id: &str) -> TokenRecord {
        TokenRecord {
            id: TokenId::new(id),
            label: Some("ci".to_owned()),
            capabilities: CapabilitySet::empty()
                .with(Capability::new(OperationId::new("server.inspect"))),
            denied: vec![OperationId::new("disk.wipe")],
            max_risk: RiskLevel::Medium,
            confirm_above: RiskLevel::High,
            scope_any: false,
            scope_servers: vec![ServerId::new("srv-prod-1")],
            created_unix: 1_000_000,
            expires_unix: 2_000_000,
        }
    }

    #[test]
    fn token_round_trips_and_lists() {
        let store = StateStore::open_in_memory().unwrap();
        let rec = record("tok_a");
        store.put_token(&rec).unwrap();

        let fetched = store.get_token(&TokenId::new("tok_a")).unwrap();
        assert_eq!(fetched.as_ref(), Some(&rec));

        store.put_token(&record("tok_b")).unwrap();
        assert_eq!(store.list_tokens().unwrap().len(), 2);
    }

    #[test]
    fn missing_token_is_none() {
        let store = StateStore::open_in_memory().unwrap();
        assert!(store.get_token(&TokenId::new("nope")).unwrap().is_none());
    }

    #[test]
    fn revocation_flips_and_is_scoped_to_the_id() {
        let store = StateStore::open_in_memory().unwrap();
        let id = TokenId::new("tok_a");
        assert!(!store.is_revoked(&id).unwrap());
        store.revoke(&id, 1_500_000).unwrap();
        assert!(store.is_revoked(&id).unwrap());
        assert!(!store.is_revoked(&TokenId::new("tok_other")).unwrap());
    }

    #[test]
    fn checkpoint_put_get_delete() {
        let store = StateStore::open_in_memory().unwrap();
        store.put_checkpoint("cp1", r#"{"prev":"config"}"#).unwrap();
        assert_eq!(
            store.get_checkpoint("cp1").unwrap().as_deref(),
            Some(r#"{"prev":"config"}"#)
        );
        store.delete_checkpoint("cp1").unwrap();
        assert!(store.get_checkpoint("cp1").unwrap().is_none());
        // Deleting again is a no-op, not an error.
        store.delete_checkpoint("cp1").unwrap();
    }

    #[test]
    fn audit_head_defaults_empty_and_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.redb");

        {
            let store = StateStore::open(&path).unwrap();
            assert_eq!(store.audit_head().unwrap(), "");
            store.set_audit_head("abc123").unwrap();
            store.put_token(&record("tok_persist")).unwrap();
        }

        // Reopen the same file: state survives.
        let reopened = StateStore::open(&path).unwrap();
        assert_eq!(reopened.audit_head().unwrap(), "abc123");
        assert!(reopened
            .get_token(&TokenId::new("tok_persist"))
            .unwrap()
            .is_some());
    }
}
