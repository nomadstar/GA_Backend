use rusqlite::Connection;
use std::error::Error;
use std::fs;
use std::env;
use std::path::PathBuf;

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
        // allow alternate name
        PathBuf::from(p)
    } else {
        PathBuf::from("analithics/analytics.db")
    }
}

/// Initialize the analytics DB (create dir + sqlite file + table)
pub fn init_db() -> Result<(), Box<dyn Error>> {
    let db_path = analytics_db_path();
    if let Some(dir) = db_path.parent() {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }
    }
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS queries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            duration_ms INTEGER,
            -- parsed fields from InputParams for easy querying
            email TEXT,
            malla TEXT,
            student_ranking REAL,
            ramos_pasados TEXT,
            ramos_prioritarios TEXT,
            filtros_json TEXT,
            -- raw payloads
            request_json TEXT,
            response_json TEXT,
            client_ip TEXT
        )",
        [],
    )?;

    // Reports table: stores analysis results (query type, params, result JSON)
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
    Ok(())
}
