use actix_web::{web, HttpResponse, Responder};

pub async fn debug_pa_names_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let qm = query.into_inner();
    let porcent_file = match qm.get("porcent").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(p) => p,
        None => return HttpResponse::BadRequest().json(serde_json::json!({"error": "porcent parameter required"})),
    };

    match crate::excel::leer_porcentajes_aprobados_con_nombres(&porcent_file) {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("excel error: {}", e)})),
    }
}
