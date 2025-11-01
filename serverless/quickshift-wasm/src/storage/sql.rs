//! Storage layer abstraction and a simple SQLite backend (optional, feature="sql").
//!
//! This module provides a tiny `StorageBackend` trait and a sqlite implementation
//! that stores named JSON blobs per key. It's intentionally lightweight so it can be
//! used to persist preprocessed tables derived from Excel parsing.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Minimal representation of a table row as JSON value.
pub type Row = Value;

/// Storage backend trait. Implementations should be provided for the target env.
pub trait StorageBackend {
    /// Save a JSON value under `key` within `table`.
    fn save(&mut self, table: &str, key: &str, value: &Row) -> Result<(), Box<dyn std::error::Error>>;
    /// Load a JSON value by table/key.
    fn load(&self, table: &str, key: &str) -> Result<Option<Row>, Box<dyn std::error::Error>>;
}

// SQLite implementation (non-wasm)
#[cfg(all(feature = "sql", not(target_arch = "wasm32")))]
pub mod sqlite {
    use super::*;
    use rusqlite::{params, Connection, NO_PARAMS};

    pub struct SqliteStorage {
        conn: Connection,
    }

    impl SqliteStorage {
        /// Open an in-memory or file-backed sqlite database.
        pub fn open(path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
            let conn = match path {
                Some(p) => Connection::open(p)?,
                None => Connection::open_in_memory()?,
            };
            Ok(SqliteStorage { conn })
        }

        fn ensure_table(&self, table: &str) -> Result<(), Box<dyn std::error::Error>> {
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                sanitize_table_name(table)
            );
            self.conn.execute(&sql, NO_PARAMS)?;
            Ok(())
        }
    }

    fn sanitize_table_name(name: &str) -> String {
        // Very small sanitizer: keep alphanum and underscore only, else replace with underscore.
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
    }

    impl StorageBackend for SqliteStorage {
        fn save(&mut self, table: &str, key: &str, value: &Row) -> Result<(), Box<dyn std::error::Error>> {
            self.ensure_table(table)?;
            let json = serde_json::to_string(value)?;
            let sql = format!("REPLACE INTO {} (key, value) VALUES (?1, ?2)", sanitize_table_name(table));
            self.conn.execute(&sql, params![key, json])?;
            Ok(())
        }

        fn load(&self, table: &str, key: &str) -> Result<Option<Row>, Box<dyn std::error::Error>> {
            self.ensure_table(table)?;
            let sql = format!("SELECT value FROM {} WHERE key = ?1", sanitize_table_name(table));
            let mut stmt = self.conn.prepare(&sql)?;
            let mut rows = stmt.query_map(params![key], |row| row.get::<_, String>(0))?;
            if let Some(res) = rows.next() {
                let s = res?;
                let v: Row = serde_json::from_str(&s)?;
                Ok(Some(v))
            } else {
                Ok(None)
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn sqlite_save_load() {
            let mut db = SqliteStorage::open(None).unwrap();
            let table = "test_table";
            let key = "row1";
            let value = json!({"a": 1, "b": "x"});
            db.save(table, key, &value).unwrap();
            let got = db.load(table, key).unwrap().unwrap();
            assert_eq!(got, value);
        }
    }
}

// When feature 'sql' is disabled or compiling to wasm, provide stubs that return errors.
#[cfg(not(all(feature = "sql", not(target_arch = "wasm32"))))]
pub mod sqlite {
    use super::*;

    pub struct SqliteStorage;

    impl SqliteStorage {
        pub fn open(_path: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
            Err("Sqlite storage not available: feature 'sql' disabled or target is wasm".into())
        }
    }

    impl StorageBackend for SqliteStorage {
        fn save(&mut self, _table: &str, _key: &str, _value: &Row) -> Result<(), Box<dyn std::error::Error>> {
            Err("Sqlite storage not available: feature 'sql' disabled or target is wasm".into())
        }

        fn load(&self, _table: &str, _key: &str) -> Result<Option<Row>, Box<dyn std::error::Error>> {
            Err("Sqlite storage not available: feature 'sql' disabled or target is wasm".into())
        }
    }
}
