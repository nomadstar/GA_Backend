use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;

#[derive(Serialize)]
struct CacheStatsRow {
    id: i64,
    ts: String,
    hits: i64,
    misses: i64,
    entries: i64,
}

pub async fn cache_stats_latest() -> impl Responder {
    match crate::analithics::db::open_analytics_connection() {
        Ok(mut conn) => match crate::analithics::db::fetch_latest_cache_stats(&mut conn) {
            Ok(Some((id, ts, hits, misses, entries))) => {
                let row = CacheStatsRow { id, ts, hits, misses, entries };
                HttpResponse::Ok().json(row)
            }
            Ok(None) => HttpResponse::Ok().json(serde_json::json!({"message":"no stats"})),
            Err(e) => {
                eprintln!("error fetching cache stats: {}", e);
                HttpResponse::InternalServerError().body("error fetching cache stats")
            }
        },
        Err(e) => {
            eprintln!("error opening analytics conn: {}", e);
            HttpResponse::InternalServerError().body("error opening analytics connection")
        }
    }
}

/// Query param: ?limit=10
pub async fn cache_stats_recent(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let lim = query.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(10) as i64;
    match crate::analithics::db::open_analytics_connection() {
        Ok(mut conn) => match crate::analithics::db::fetch_recent_cache_stats(&mut conn, lim) {
            Ok(rows) => {
                let out: Vec<CacheStatsRow> = rows.into_iter().map(|(id, ts, hits, misses, entries)| CacheStatsRow { id, ts, hits, misses, entries }).collect();
                HttpResponse::Ok().json(out)
            }
            Err(e) => {
                eprintln!("error fetching recent cache stats: {}", e);
                HttpResponse::InternalServerError().body("error fetching recent cache stats")
            }
        },
        Err(e) => {
            eprintln!("error opening analytics conn: {}", e);
            HttpResponse::InternalServerError().body("error opening analytics connection")
        }
    }
}
