use actix_web::{HttpResponse, Responder};
use serde_json::json;

pub async fn anal_ramos_pasados_handler(query: actix_web::web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    match crate::analithics::ramos_mas_pasados(limit) {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
    }
}

pub async fn anal_ranking_handler() -> impl Responder {
    match crate::analithics::ranking_por_estudiante() {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
    }
}

pub async fn anal_count_users_handler() -> impl Responder {
    match crate::analithics::count_users() {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
    }
}

pub async fn anal_filtros_handler() -> impl Responder {
    match crate::analithics::filtros_mas_solicitados() {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
    }
}

pub async fn anal_ramos_recomendados_handler(query: actix_web::web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    match crate::analithics::ramos_mas_recomendados(limit) {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
    }
}
