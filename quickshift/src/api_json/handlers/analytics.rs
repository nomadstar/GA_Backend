use actix_web::{HttpResponse, Responder, web};
use serde_json::json;

pub async fn anal_ramos_pasados_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    let res = web::block(move || crate::analithics::ramos_mas_pasados(limit).map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_ranking_handler() -> impl Responder {
    let res = web::block(|| crate::analithics::ranking_por_estudiante().map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_count_users_handler() -> impl Responder {
    let res = web::block(|| crate::analithics::count_users().map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_filtros_handler() -> impl Responder {
    let res = web::block(|| crate::analithics::filtros_mas_solicitados().map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_ramos_recomendados_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    let res = web::block(move || crate::analithics::ramos_mas_recomendados(limit).map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_horarios_recomendados_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let limit = query.get("limit").and_then(|s| s.parse::<usize>().ok());
    let res = web::block(move || crate::analithics::horarios_mas_recomendados(limit).map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_profesores_handler() -> impl Responder {
    let res = web::block(|| crate::analithics::profesores_y_cursos().map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}

pub async fn anal_cursos_por_malla_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let malla = match query.get("malla") {
        Some(s) => s.clone(),
        None => return HttpResponse::BadRequest().json(json!({"error": "missing malla parameter"})),
    };
    let res = web::block(move || crate::analithics::cursos_por_malla(&malla).map_err(|e| format!("{}", e))).await;
    match res {
        Ok(Ok(v)) => HttpResponse::Ok().json(v),
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("analytics error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("blocking task error: {}", e)})),
    }
}
