use rusqlite::{params, Connection};
use std::error::Error;
use std::fs;
use std::env;
use std::path::PathBuf;
use std::fmt;

// Postgres client for remote DB support
use postgres::{Client, NoTls};

/// Abstracción sencilla para conexiones de analytics que puede ser SQLite o Postgres
pub enum AnalyticsConn {
    Sqlite(Connection),
    Postgres(Client),
}

impl fmt::Debug for AnalyticsConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyticsConn::Sqlite(_) => write!(f, "AnalyticsConn::Sqlite(..)"),
            AnalyticsConn::Postgres(_) => write!(f, "AnalyticsConn::Postgres(..)"),
        }
    }
}

// load .env at module init if present
fn load_dotenv() {
    let _ = dotenv::dotenv();
}

/// Return the path to the analytics DB. Exposed so other submodules can open
/// short-lived connections. Honors ANALITHICS_DB_PATH / ANALITHICS_DB_URL env.
pub fn analytics_db_path() -> PathBuf {
    load_dotenv();
    if let Ok(p) = env::var("ANALITHICS_DB_PATH") {
        PathBuf::from(p)
    } else if let Ok(p) = env::var("ANALITHICS_DB_URL") {
        // If the URL starts with sqlite:// or file://, strip the scheme and return path
        if p.starts_with("sqlite://") {
            // support sqlite:///absolute/path and sqlite://relative/path
            let without = p.trim_start_matches("sqlite://");
            PathBuf::from(without)
        } else if p.starts_with("file://") {
            let without = p.trim_start_matches("file://");
            PathBuf::from(without)
        } else {
            // For remote DB URLs (postgres://...) we can't produce a local PathBuf; return default path
            PathBuf::from("analithics/analytics.db")
        }
    } else {
        PathBuf::from("analithics/analytics.db")
    }
}

/// Initialize the analytics DB (create dir + sqlite file + table)
pub fn init_db() -> Result<(), Box<dyn Error>> {
    load_dotenv();
    // If using a local file-based sqlite, ensure directory exists
    if let Ok(url) = env::var("ANALITHICS_DB_URL") {
        if url.starts_with("sqlite://") || url.starts_with("file://") || env::var("ANALITHICS_DB_PATH").is_ok()
        {
            let db_path = analytics_db_path();
            if let Some(dir) = db_path.parent() {
                if !dir.exists() {
                    fs::create_dir_all(dir)?;
                }
            }
        }
    }

    // Open a connection (either sqlite or postgres) and ensure tables exist
    match open_analytics_connection() {
        Ok(AnalyticsConn::Sqlite(conn)) => {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS queries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts TEXT NOT NULL,
                    duration_ms INTEGER,
                    email TEXT,
                    malla TEXT,
                    student_ranking REAL,
                    ramos_pasados TEXT,
                    ramos_prioritarios TEXT,
                    filtros_json TEXT,
                    request_json TEXT,
                    response_json TEXT,
                    client_ip TEXT
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS reports (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts TEXT NOT NULL,
                    query_type TEXT NOT NULL,
                    params_json TEXT,
                    result_json TEXT
                )",
                [],
            )?;

            conn.execute(
                "CREATE TABLE IF NOT EXISTS cache_stats (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    ts TEXT NOT NULL,
                    hits INTEGER,
                    misses INTEGER,
                    entries INTEGER
                )",
                [],
            )?;
            Ok(())
        }
        Ok(AnalyticsConn::Postgres(mut client)) => {
            // Use PostgreSQL types; id serial primary key, ts text (or timestamptz), hits bigint
            client.batch_execute(
                "CREATE TABLE IF NOT EXISTS queries (
                    id BIGSERIAL PRIMARY KEY,
                    ts TEXT NOT NULL,
                    duration_ms BIGINT,
                    email TEXT,
                    malla TEXT,
                    student_ranking DOUBLE PRECISION,
                    ramos_pasados TEXT,
                    ramos_prioritarios TEXT,
                    filtros_json TEXT,
                    request_json TEXT,
                    response_json TEXT,
                    client_ip TEXT
                );

                CREATE TABLE IF NOT EXISTS reports (
                    id BIGSERIAL PRIMARY KEY,
                    ts TEXT NOT NULL,
                    query_type TEXT NOT NULL,
                    params_json TEXT,
                    result_json TEXT
                );

                CREATE TABLE IF NOT EXISTS cache_stats (
                    id BIGSERIAL PRIMARY KEY,
                    ts TEXT NOT NULL,
                    hits BIGINT,
                    misses BIGINT,
                    entries BIGINT
                );",
            )?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Open a connection to the analytics DB, accepting sqlite:// URLs or plain paths.
/// Open a connection to the analytics DB. Accepts sqlite://, file:// and postgres:// URLs.
pub fn open_analytics_connection() -> Result<AnalyticsConn, Box<dyn Error>> {
    load_dotenv();
    if let Ok(url) = env::var("ANALITHICS_DB_URL") {
        if url.starts_with("sqlite://") {
            let path = url.trim_start_matches("sqlite://");
            let conn = Connection::open(path)?;
            return Ok(AnalyticsConn::Sqlite(conn));
        } else if url.starts_with("file://") {
            let path = url.trim_start_matches("file://");
            let conn = Connection::open(path)?;
            return Ok(AnalyticsConn::Sqlite(conn));
        } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            // Connect to remote Postgres
            let client = Client::connect(&url, NoTls)?;
            return Ok(AnalyticsConn::Postgres(client));
        } else {
            return Err(format!("ANALITHICS_DB_URL uses unsupported scheme: {}", url).into());
        }
    }

    // Fallback to ANALITHICS_DB_PATH or default path -> sqlite
    let path = analytics_db_path();
    let conn = Connection::open(path)?;
    Ok(AnalyticsConn::Sqlite(conn))
}

/// Record cache stats into cache_stats table
pub fn record_cache_stats(conn: &mut AnalyticsConn, ts: &str, hits: i64, misses: i64, entries: i64) -> Result<(), Box<dyn Error>> {
    match conn {
        AnalyticsConn::Sqlite(c) => {
            c.execute(
                "INSERT INTO cache_stats (ts, hits, misses, entries) VALUES (?1, ?2, ?3, ?4)",
                params![ts, hits, misses, entries],
            )?;
            Ok(())
        }
        AnalyticsConn::Postgres(client) => {
            // $1.. parameterized insert
            client.execute(
                "INSERT INTO cache_stats (ts, hits, misses, entries) VALUES ($1, $2, $3, $4)",
                &[&ts, &hits, &misses, &entries],
            )?;
            Ok(())
        }
    }
}
