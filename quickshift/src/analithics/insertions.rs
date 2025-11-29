use crate::analithics::db::{open_analytics_connection, AnalyticsConn};
use crate::analithics::jsonparsing::extract_parsed_fields;
use rusqlite::params;
use postgres::NoTls;
use chrono::Utc;
use std::error::Error;

/// Insert a query row into the analytics DB. Uses `extract_parsed_fields` to
/// populate the parsed columns when possible. This function opens a short-lived
/// connection and inserts the row.
pub fn log_query(request_json: &str, response_json: &str, duration_ms: i64, client_ip: &str) -> Result<(), Box<dyn Error>> {
    let ts = Utc::now().to_rfc3339();

    // best-effort parse
    let parsed = extract_parsed_fields(request_json)?;

    // Open analytics conn and branch
    let conn = open_analytics_connection()?;
    match conn {
        AnalyticsConn::Sqlite(c) => {
            c.execute(
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
        AnalyticsConn::PostgresConfig(url) => {
            let url = url.clone();
            let ts_s = ts.clone();
            let request_s = request_json.to_string();
            let response_s = response_json.to_string();
            let parsed_email = parsed.email;
            let parsed_malla = parsed.malla;
            let parsed_student_ranking = parsed.student_ranking;
            let parsed_ramos_pasados = parsed.ramos_pasados;
            let parsed_ramos_prioritarios = parsed.ramos_prioritarios;
            let parsed_filtros_json = parsed.filtros_json;
            let client_ip_s = client_ip.to_string();

            let handle = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + 'static>> {
                let mut client = postgres::Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                client.execute(
                    "INSERT INTO queries (ts, duration_ms, email, malla, student_ranking, ramos_pasados, ramos_prioritarios, filtros_json, request_json, response_json, client_ip) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
                    &[&ts_s, &duration_ms, &parsed_email, &parsed_malla, &parsed_student_ranking, &parsed_ramos_pasados, &parsed_ramos_prioritarios, &parsed_filtros_json, &request_s, &response_s, &client_ip_s],
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

/// Save an analysis result under `reports` table.
pub fn save_report(query_type: &str, params_json: &str, result_json: &str) -> Result<(), Box<dyn Error>> {
    let ts = Utc::now().to_rfc3339();
    let conn = open_analytics_connection()?;
    match conn {
        AnalyticsConn::Sqlite(c) => {
            c.execute(
                "INSERT INTO reports (ts, query_type, params_json, result_json) VALUES (?1, ?2, ?3, ?4)",
                params![ts, query_type, params_json, result_json],
            )?;
            Ok(())
        }
        AnalyticsConn::PostgresConfig(url) => {
            let url = url.clone();
            let ts_s = ts.clone();
            let q = query_type.to_string();
            let p = params_json.to_string();
            let r = result_json.to_string();
            let handle = std::thread::spawn(move || -> Result<(), Box<dyn Error + Send + 'static>> {
                let mut client = postgres::Client::connect(&url, NoTls).map_err(|e| Box::new(e) as Box<dyn Error + Send + 'static>)?;
                client.execute(
                    "INSERT INTO reports (ts, query_type, params_json, result_json) VALUES ($1,$2,$3,$4)",
                    &[&ts_s, &q, &p, &r],
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
