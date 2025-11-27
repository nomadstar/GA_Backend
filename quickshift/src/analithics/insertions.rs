use crate::analithics::db::analytics_db_path;
use crate::analithics::jsonparsing::extract_parsed_fields;
use rusqlite::{params, Connection};
use chrono::Utc;
use std::error::Error;

/// Insert a query row into the analytics DB. Uses `extract_parsed_fields` to
/// populate the parsed columns when possible. This function opens a short-lived
/// connection and inserts the row.
pub fn log_query(request_json: &str, response_json: &str, duration_ms: i64, client_ip: &str) -> Result<(), Box<dyn Error>> {
    let db_path = analytics_db_path();
    let conn = Connection::open(db_path)?;
    let ts = Utc::now().to_rfc3339();

    // best-effort parse
    let parsed = extract_parsed_fields(request_json)?;

    conn.execute(
        "INSERT INTO queries (
            ts, duration_ms, email, malla, student_ranking,
            ramos_pasados, ramos_prioritarios, filtros_json,
            request_json, response_json, client_ip
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            ts,
            duration_ms,
            parsed.email,
            parsed.malla,
            parsed.student_ranking,
            parsed.ramos_pasados,
            parsed.ramos_prioritarios,
            parsed.filtros_json,
            request_json,
            response_json,
            client_ip,
        ],
    )?;
    Ok(())
}

/// Save an analysis result under `reports` table.
pub fn save_report(query_type: &str, params_json: &str, result_json: &str) -> Result<(), Box<dyn Error>> {
    let db_path = analytics_db_path();
    let conn = Connection::open(db_path)?;
    let ts = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO reports (ts, query_type, params_json, result_json) VALUES (?1, ?2, ?3, ?4)",
        params![ts, query_type, params_json, result_json],
    )?;
    Ok(())
}
