use rusqlite::{params, Connection};
use std::error::Error;
use std::fs;
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::env;
use std::path::PathBuf;

// load .env at module init if present
fn load_dotenv() {
    let _ = dotenv::dotenv();
}

fn analytics_db_path() -> PathBuf {
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

// We will try to parse the incoming request JSON into the crate's
// `InputParams` so we can persist some fields in separate columns for
// easier querying. Parsing is best-effort: if it fails we still store
// the raw JSON in `request_json` and insert NULLs for parsed columns.
use crate::api_json::InputParams;

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

/// Log a single query to the DB. This opens a short-lived connection and inserts the row.
///
/// The function accepts the raw `request_json` and will attempt to parse it
/// into `InputParams` to extract `email`, `malla`, `student_ranking`, and the
/// two ramo lists. If parsing fails the parsed columns will be NULL but the
/// raw JSON will still be saved.
pub fn log_query(request_json: &str, response_json: &str, duration_ms: i64, client_ip: &str) -> Result<(), Box<dyn Error>> {
    let db_path = analytics_db_path();
    let conn = Connection::open(db_path)?;

    // timestamp
    let ts = Utc::now().to_rfc3339();

    // Attempt to parse and extract common fields
    let mut email: Option<String> = None;
    let mut malla: Option<String> = None;
    let mut student_ranking: Option<f64> = None;
    let mut ramos_pasados: Option<String> = None; // store as JSON string
    let mut ramos_prioritarios: Option<String> = None; // store as JSON string
    let mut filtros_json: Option<String> = None;

    if let Ok(parsed) = serde_json::from_str::<InputParams>(request_json) {
        email = Some(parsed.email);
        malla = Some(parsed.malla);
        student_ranking = parsed.student_ranking;

        // serialize the ramo vectors as compact JSON strings for storage
        if !parsed.ramos_pasados.is_empty() {
            if let Ok(s) = serde_json::to_string(&parsed.ramos_pasados) {
                ramos_pasados = Some(s);
            }
        }
        if !parsed.ramos_prioritarios.is_empty() {
            if let Ok(s) = serde_json::to_string(&parsed.ramos_prioritarios) {
                ramos_prioritarios = Some(s);
            }
        }

        if let Some(f) = parsed.filtros {
            // store filtros as JSON; it's serde-friendly via UserFilters
            if let Ok(j) = serde_json::to_string(&f) {
                filtros_json = Some(j);
            }
        }
    } else {
        // If parsing as InputParams fails we still try to extract a few
        // fields heuristically (best-effort) from the raw JSON.
        if let Ok(v) = serde_json::from_str::<JsonValue>(request_json) {
            if let Some(e) = v.get("email").and_then(|x| x.as_str()) {
                email = Some(e.to_string());
            }
            if let Some(m) = v.get("malla").and_then(|x| x.as_str()) {
                malla = Some(m.to_string());
            }
            if let Some(sr) = v.get("student_ranking").and_then(|x| x.as_f64()) {
                student_ranking = Some(sr);
            }
            if let Some(rp) = v.get("ramos_pasados") {
                if let Ok(s) = serde_json::to_string(rp) { ramos_pasados = Some(s); }
            }
            if let Some(rp) = v.get("ramos_prioritarios") {
                if let Ok(s) = serde_json::to_string(rp) { ramos_prioritarios = Some(s); }
            }
            if let Some(f) = v.get("filtros") {
                if let Ok(s) = serde_json::to_string(f) { filtros_json = Some(s); }
            }
        }
    }

    conn.execute(
        "INSERT INTO queries (
            ts, duration_ms, email, malla, student_ranking,
            ramos_pasados, ramos_prioritarios, filtros_json,
            request_json, response_json, client_ip
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            ts,
            duration_ms,
            email,
            malla,
            student_ranking,
            ramos_pasados,
            ramos_prioritarios,
            filtros_json,
            request_json,
            response_json,
            client_ip,
        ],
    )?;

    Ok(())
}

/// Save an analysis report into the `reports` table.
pub fn save_report(query_type: &str, params_json: &str, result_json: &str) -> Result<(), Box<dyn Error>> {
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let ts = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO reports (ts, query_type, params_json, result_json) VALUES (?1, ?2, ?3, ?4)",
        params![ts, query_type, params_json, result_json],
    )?;
    Ok(())
}

/// Return a JSON array with the most passed courses across all recorded queries.
/// Result: [{"ramo": "CBM1001", "count": 42}, ...]
pub fn ramos_mas_pasados(limit: Option<usize>) -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT ramos_pasados FROM queries WHERE ramos_pasados IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in rows {
        if let Ok(s) = r {
            if let Ok(vec) = serde_json::from_str::<Vec<String>>(&s) {
                for code in vec {
                    *counts.entry(code).or_default() += 1;
                }
            }
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    let lim = limit.unwrap_or(20);
    let arr: Vec<serde_json::Value> = v.into_iter().take(lim).map(|(r, c)| serde_json::json!({"ramo": r, "count": c})).collect();
    let result = serde_json::Value::Array(arr);
    // persist report
    let params = serde_json::json!({"limit": limit});
    let _ = save_report("ramos_mas_pasados", &params.to_string(), &result.to_string());
    Ok(result)
}

/// Return latest assumed ranking per student (by email). Result: [{"email":..., "student_ranking":...}, ...]
pub fn ranking_por_estudiante() -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    use chrono::DateTime;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT email, student_ranking, ts FROM queries WHERE email IS NOT NULL AND student_ranking IS NOT NULL")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, String>(2)?)))?;
    let mut latest: HashMap<String, (f64, DateTime<Utc>)> = HashMap::new();
    for r in rows {
        if let Ok((email, rank, ts)) = r {
            if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
                match latest.get(&email) {
                    Some((_, existing_dt)) => {
                        if &dt > existing_dt {
                            latest.insert(email, (rank, dt));
                        }
                    }
                    None => { latest.insert(email, (rank, dt)); }
                }
            }
        }
    }
    let arr: Vec<serde_json::Value> = latest.into_iter().map(|(e, (r, _))| serde_json::json!({"email": e, "student_ranking": r})).collect();
    let result = serde_json::Value::Array(arr);
    let _ = save_report("ranking_por_estudiante", "{}", &result.to_string());
    Ok(result)
}

/// Count distinct users (by email)
pub fn count_users() -> Result<serde_json::Value, Box<dyn Error>> {
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT DISTINCT email FROM queries WHERE email IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut cnt: usize = 0;
    for _ in rows { cnt += 1; }
    let result = serde_json::json!({"count_users": cnt});
    let _ = save_report("count_users", "{}", &result.to_string());
    Ok(result)
}

/// Analyze which filters are most requested. Returns counts per filter type.
pub fn filtros_mas_solicitados() -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT filtros_json FROM queries WHERE filtros_json IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in rows {
        if let Ok(s) = r {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(dhl) = v.get("dias_horarios_libres") {
                    if dhl.get("habilitado").and_then(|x| x.as_bool()).unwrap_or(false) {
                        *counts.entry("dias_horarios_libres".to_string()).or_default() += 1;
                    }
                }
                if let Some(vent) = v.get("ventana_entre_actividades") {
                    if vent.get("habilitado").and_then(|x| x.as_bool()).unwrap_or(false) {
                        *counts.entry("ventana_entre_actividades".to_string()).or_default() += 1;
                    }
                }
                if let Some(pref) = v.get("preferencias_profesores") {
                    if pref.get("habilitado").and_then(|x| x.as_bool()).unwrap_or(false) {
                        *counts.entry("preferencias_profesores".to_string()).or_default() += 1;
                    }
                }
                if let Some(bal) = v.get("balance_lineas") {
                    if bal.get("habilitado").and_then(|x| x.as_bool()).unwrap_or(false) {
                        *counts.entry("balance_lineas".to_string()).or_default() += 1;
                    }
                }
            }
        }
    }
    let mut vec: Vec<(String, usize)> = counts.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let arr: Vec<serde_json::Value> = vec.into_iter().map(|(k, c)| serde_json::json!({"filter": k, "count": c})).collect();
    let result = serde_json::Value::Array(arr);
    let _ = save_report("filtros_mas_solicitados", "{}", &result.to_string());
    Ok(result)
}

/// Determine the most recommended ramos by scanning stored `response_json` fields.
/// Looks for top-level "soluciones" arrays and extracts candidate strings that look like course codes.
pub fn ramos_mas_recomendados(limit: Option<usize>) -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT response_json FROM queries WHERE response_json IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in rows {
        if let Ok(s) = r {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                if let Some(soluciones) = v.get("soluciones").and_then(|x| x.as_array()) {
                    for sol in soluciones {
                        // gather strings recursively inside sol
                        extract_codes_from_value(sol, &mut counts);
                    }
                } else {
                    // fallback: scan whole JSON for strings that look like codes
                    extract_codes_from_value(&v, &mut counts);
                }
            }
        }
    }
    let mut vec: Vec<(String, usize)> = counts.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let lim = limit.unwrap_or(20);
    let arr: Vec<serde_json::Value> = vec.into_iter().take(lim).map(|(r, c)| serde_json::json!({"ramo": r, "count": c})).collect();
    let result = serde_json::Value::Array(arr);
    let params = serde_json::json!({"limit": limit});
    let _ = save_report("ramos_mas_recomendados", &params.to_string(), &result.to_string());
    Ok(result)
}

fn extract_codes_from_value(v: &serde_json::Value, counts: &mut std::collections::HashMap<String, usize>) {
    match v {
        serde_json::Value::String(s) => {
            // heuristic: strings with at least one digit and length > 2
            if s.chars().any(|c| c.is_ascii_digit()) && s.len() > 2 {
                *counts.entry(s.clone()).or_default() += 1;
            }
        }
        serde_json::Value::Array(arr) => {
            for it in arr { extract_codes_from_value(it, counts); }
        }
        serde_json::Value::Object(map) => {
            for (_k, val) in map { extract_codes_from_value(val, counts); }
        }
        _ => {}
    }
}
