use rusqlite::Connection;
use std::error::Error;
use serde_json::Value as JsonValue;
use chrono::Utc;

/// Return a JSON array with the most passed courses across all recorded queries.
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
    let _ = crate::analithics::save_report("ramos_mas_pasados", &params.to_string(), &result.to_string());
    Ok(result)
}

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
    let _ = crate::analithics::save_report("ranking_por_estudiante", "{}", &result.to_string());
    Ok(result)
}

pub fn count_users() -> Result<serde_json::Value, Box<dyn Error>> {
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT DISTINCT email FROM queries WHERE email IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut cnt: usize = 0;
    for _ in rows { cnt += 1; }
    let result = serde_json::json!({"count_users": cnt});
    let _ = crate::analithics::save_report("count_users", "{}", &result.to_string());
    Ok(result)
}

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
    let _ = crate::analithics::save_report("filtros_mas_solicitados", "{}", &result.to_string());
    Ok(result)
}

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
                    for sol in soluciones { extract_codes_from_value(sol, &mut counts); }
                } else { extract_codes_from_value(&v, &mut counts); }
            }
        }
    }
    let mut vec: Vec<(String, usize)> = counts.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let lim = limit.unwrap_or(20);
    let arr: Vec<serde_json::Value> = vec.into_iter().take(lim).map(|(r, c)| serde_json::json!({"ramo": r, "count": c})).collect();
    let result = serde_json::Value::Array(arr);
    let params = serde_json::json!({"limit": limit});
    let _ = crate::analithics::save_report("ramos_mas_recomendados", &params.to_string(), &result.to_string());
    Ok(result)
}

fn extract_codes_from_value(v: &serde_json::Value, counts: &mut std::collections::HashMap<String, usize>) {
    match v {
        serde_json::Value::String(s) => {
            if s.chars().any(|c| c.is_ascii_digit()) && s.len() > 2 { *counts.entry(s.clone()).or_default() += 1; }
        }
        serde_json::Value::Array(arr) => { for it in arr { extract_codes_from_value(it, counts); } }
        serde_json::Value::Object(map) => { for (_k, val) in map { extract_codes_from_value(val, counts); } }
        _ => {}
    }
}

pub fn tasa_aprobacion_por_ramo(limit: Option<usize>) -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    use chrono::DateTime;
    use chrono::Utc;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT email, ramos_pasados, ts FROM queries WHERE email IS NOT NULL AND ramos_pasados IS NOT NULL")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))?;
    let mut latest: HashMap<String, (String, DateTime<Utc>)> = HashMap::new();
    for r in rows {
        if let Ok((email, ramos_json, ts)) = r {
            if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
                match latest.get(&email) {
                    Some((_, existing_dt)) => { if &dt > existing_dt { latest.insert(email, (ramos_json, dt)); } }
                    None => { latest.insert(email, (ramos_json, dt)); }
                }
            }
        }
    }
    let total_students = latest.len();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (_email, (ramos_json, _dt)) in latest.into_iter() {
        if let Ok(vec) = serde_json::from_str::<Vec<String>>(&ramos_json) {
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            for code in vec { if seen.insert(code.clone()) { *counts.entry(code).or_default() += 1; } }
        }
    }
    let mut v: Vec<(String, usize)> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    let lim = limit.unwrap_or(50);
    let arr: Vec<serde_json::Value> = v.into_iter().take(lim).map(|(r, c)| {
        let pass_rate = if total_students > 0 { (c as f64) / (total_students as f64) } else { 0.0 };
        serde_json::json!({"ramo": r, "passed_students": c, "total_students": total_students, "pass_rate": pass_rate})
    }).collect();
    let result = serde_json::Value::Array(arr);
    let params = serde_json::json!({"limit": limit});
    let _ = crate::analithics::save_report("tasa_aprobacion_por_ramo", &params.to_string(), &result.to_string());
    Ok(result)
}

pub fn promedio_ranking_y_stddev() -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    use chrono::DateTime;
    use chrono::Utc;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT email, student_ranking, ts FROM queries WHERE email IS NOT NULL AND student_ranking IS NOT NULL")?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?, row.get::<_, String>(2)?)))?;
    let mut latest: HashMap<String, (f64, DateTime<Utc>)> = HashMap::new();
    for r in rows {
        if let Ok((email, rank, ts)) = r {
            if let Ok(dt) = ts.parse::<DateTime<Utc>>() {
                match latest.get(&email) {
                    Some((_, existing_dt)) => { if &dt > existing_dt { latest.insert(email, (rank, dt)); } }
                    None => { latest.insert(email, (rank, dt)); }
                }
            }
        }
    }
    let n = latest.len();
    let mut sum = 0.0f64; for (_e, (r, _)) in latest.iter() { sum += *r; }
    let mean = if n > 0 { sum / (n as f64) } else { 0.0 };
    let mut var_sum = 0.0f64; for (_e, (r, _)) in latest.iter() { var_sum += (r - mean)*(r - mean); }
    let variance = if n > 0 { var_sum / (n as f64) } else { 0.0 };
    let stddev = variance.sqrt();
    let result = serde_json::json!({"n": n, "mean": mean, "stddev": stddev});
    let _ = crate::analithics::save_report("promedio_ranking_y_stddev", "{}", &result.to_string());
    Ok(result)
}

pub fn horarios_mas_ocupados(limit: Option<usize>) -> Result<serde_json::Value, Box<dyn Error>> {
    use std::collections::HashMap;
    let db_path = std::path::Path::new("analithics").join("analytics.db");
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare("SELECT response_json FROM queries WHERE response_json IS NOT NULL")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut counts: HashMap<String, usize> = HashMap::new();
    for r in rows {
        if let Ok(s) = r {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                extract_horarios_from_value(&v, &mut counts);
            }
        }
    }
    let mut vec: Vec<(String, usize)> = counts.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1));
    let lim = limit.unwrap_or(20);
    let arr: Vec<serde_json::Value> = vec.into_iter().take(lim).map(|(h, c)| serde_json::json!({"horario": h, "count": c})).collect();
    let result = serde_json::Value::Array(arr);
    let params = serde_json::json!({"limit": limit});
    let _ = crate::analithics::save_report("horarios_mas_ocupados", &params.to_string(), &result.to_string());
    Ok(result)
}

fn extract_horarios_from_value(v: &serde_json::Value, counts: &mut std::collections::HashMap<String, usize>) {
    match v {
        serde_json::Value::Object(map) => {
            if let Some(hv) = map.get("horario") {
                match hv {
                    serde_json::Value::String(s) => { if !s.is_empty() { *counts.entry(s.clone()).or_default() += 1; } }
                    serde_json::Value::Array(arr) => { for it in arr { if let serde_json::Value::String(s) = it { if !s.is_empty() { *counts.entry(s.clone()).or_default() += 1; } } } }
                    _ => {}
                }
            }
            for (_k, val) in map { extract_horarios_from_value(val, counts); }
        }
        serde_json::Value::Array(arr) => { for it in arr { extract_horarios_from_value(it, counts); } }
        _ => {}
    }
}
