use rusqlite::{params, Connection};
use std::error::Error;
use std::fs;
use std::env;
use std::path::PathBuf;
use std::fmt;

// Postgres client for remote DB support
use postgres::{Client, NoTls};

/// Abstracción sencilla para conexiones de analytics que puede ser SQLite o Postgres.
/// Para Postgres guardamos la URL y realizamos operaciones en un hilo separado
/// para evitar intentar arrancar runtimes tokio dentro del runtime existente.
pub enum AnalyticsConn {
    Sqlite(Connection),
    /// Contiene la URL completa (postgres://...)
    PostgresConfig(String),
}

impl fmt::Debug for AnalyticsConn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AnalyticsConn::Sqlite(_) => write!(f, "AnalyticsConn::Sqlite(..)"),
            AnalyticsConn::PostgresConfig(_) => write!(f, "AnalyticsConn::PostgresConfig(..)"),
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
        Ok(AnalyticsConn::PostgresConfig(url)) => {
            // Run table creation in a dedicated thread to avoid runtime conflicts
            let url = url.clone();
            let handle = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + 'static>> {
                let mut client = Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
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
                ).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                Ok(())
            });
            match handle.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e as Box<dyn Error>),
                Err(e) => Err(format!("thread join error: {:?}", e).into()),
            }
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
            // For Postgres we only keep the URL and defer actual connect to
            // the operation site (init_db / record_cache_stats). This avoids
            // trying to start a tokio runtime inside the Actix runtime.
            return Ok(AnalyticsConn::PostgresConfig(url));
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
pub fn record_cache_stats(conn: &AnalyticsConn, ts: &str, hits: i64, misses: i64, entries: i64) -> Result<(), Box<dyn Error>> {
    match conn {
        AnalyticsConn::Sqlite(c) => {
            c.execute(
                "INSERT INTO cache_stats (ts, hits, misses, entries) VALUES (?1, ?2, ?3, ?4)",
                params![ts, hits, misses, entries],
            )?;
            Ok(())
        }
        AnalyticsConn::PostgresConfig(url) => {
            // Perform the insert in a separate thread to avoid blocking/rt issues
            let url = url.clone();
            let ts_s = ts.to_string();
            let handle = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + 'static>> {
                let mut client = Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                client.execute(
                    "INSERT INTO cache_stats (ts, hits, misses, entries) VALUES ($1, $2, $3, $4)",
                    &[&ts_s, &hits, &misses, &entries],
                ).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                Ok(())
            });
            match handle.join() {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e as Box<dyn Error>),
                Err(e) => Err(format!("thread join error: {:?}", e).into()),
            }
        }
    }
}

/// Fetch the latest cache_stats row (by id desc)
pub fn fetch_latest_cache_stats(conn: &AnalyticsConn) -> Result<Option<(i64, String, i64, i64, i64)>, Box<dyn Error>> {
    match conn {
        AnalyticsConn::Sqlite(c) => {
            let mut stmt = c.prepare("SELECT id, ts, hits, misses, entries FROM cache_stats ORDER BY id DESC LIMIT 1")?;
            let mut rows = stmt.query([])?;
            if let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let ts: String = row.get(1)?;
                let hits: i64 = row.get(2)?;
                let misses: i64 = row.get(3)?;
                let entries: i64 = row.get(4)?;
                Ok(Some((id, ts, hits, misses, entries)))
            } else {
                Ok(None)
            }
        }
        AnalyticsConn::PostgresConfig(url) => {
            let url = url.clone();
            let handle = std::thread::spawn(move || -> Result<Option<(i64, String, i64, i64, i64)>, Box<dyn Error + Send + 'static>> {
                let mut client = Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                let rows = client.query("SELECT id, ts, hits, misses, entries FROM cache_stats ORDER BY id DESC LIMIT 1", &[]).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                if let Some(r) = rows.get(0) {
                    let id: i64 = r.get(0);
                    let ts: String = r.get(1);
                    let hits: i64 = r.get(2);
                    let misses: i64 = r.get(3);
                    let entries: i64 = r.get(4);
                    Ok(Some((id, ts, hits, misses, entries)))
                } else {
                    Ok(None)
                }
            });
            match handle.join() {
                Ok(res) => res.map_err(|e| e as Box<dyn Error>),
                Err(e) => Err(format!("thread join error: {:?}", e).into()),
            }
        }
    }
}

/// Fetch recent cache_stats rows (limit)
pub fn fetch_recent_cache_stats(conn: &AnalyticsConn, limit: i64) -> Result<Vec<(i64, String, i64, i64, i64)>, Box<dyn Error>> {
    match conn {
        AnalyticsConn::Sqlite(c) => {
            let mut stmt = c.prepare("SELECT id, ts, hits, misses, entries FROM cache_stats ORDER BY id DESC LIMIT ?1")?;
            let rows_iter = stmt.query_map(params![limit], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })?;
            let mut out = Vec::new();
            for r in rows_iter {
                out.push(r?);
            }
            Ok(out)
        }
        AnalyticsConn::PostgresConfig(url) => {
            let url = url.clone();
            let handle = std::thread::spawn(move || -> Result<Vec<(i64, String, i64, i64, i64)>, Box<dyn Error + Send + 'static>> {
                let mut client = Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                let rows = client.query("SELECT id, ts, hits, misses, entries FROM cache_stats ORDER BY id DESC LIMIT $1", &[&limit]).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                let mut out = Vec::new();
                for r in rows.iter() {
                    out.push((r.get(0), r.get(1), r.get(2), r.get(3), r.get(4)));
                }
                Ok(out)
            });
            match handle.join() {
                Ok(res) => res.map_err(|e| e as Box<dyn Error>),
                Err(e) => Err(format!("thread join error: {:?}", e).into()),
            }
        }
    }
}
