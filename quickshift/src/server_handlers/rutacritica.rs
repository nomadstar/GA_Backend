use actix_web::{web, HttpResponse, Responder};
use serde_json::json;

pub async fn rutacomoda_best_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    static GLOBAL_SEM2: std::sync::OnceLock<std::sync::Arc<tokio::sync::Semaphore>> = std::sync::OnceLock::new();
    let sem2 = GLOBAL_SEM2.get_or_init(|| std::sync::Arc::new(tokio::sync::Semaphore::new(std::cmp::max(1, num_cpus::get())))).clone();
    let permit2 = match sem2.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().json(json!({"error": "failed to acquire semaphore"})),
    };

    let blocking = tokio::task::spawn_blocking(move || {
        let _permit2 = permit2;
        match crate::algorithm::ejecutar_ruta_critica_with_params(params) {
            Ok(sol) => Ok(sol),
            Err(e) => Err(format!("{}", e)),
        }
    });

    match blocking.await {
        Ok(Ok(soluciones)) => {
            if soluciones.is_empty() {
                return HttpResponse::Ok().json(json!({"best": []}));
            }

            let mut max_score: Option<i64> = None;
            for (_sol, score) in soluciones.iter() {
                match max_score {
                    None => max_score = Some(*score),
                    Some(ms) => if *score > ms { max_score = Some(*score); }
                }
            }

            let ms = max_score.unwrap_or(0);
            let mut bests: Vec<serde_json::Value> = Vec::new();
            for (sol, score) in soluciones.into_iter() {
                if score == ms {
                    let path_codes: Vec<String> = sol.into_iter().map(|(s, _prio)| s.codigo).collect();
                    bests.push(json!({"path": path_codes, "score": score}));
                }
            }

            HttpResponse::Ok().json(json!({"best": bests}))
        }
        Ok(Err(e)) => HttpResponse::InternalServerError().json(json!({"error": format!("algorithm error: {}", e)})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("task join error: {}", e)})),
    }
}

pub async fn rutacritica_run_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    // DEBUG: incluir optimizations en response para verificar que se parsea
    let debug_info = json!({
        "optimizations_received": params.optimizations.clone(),
        "horarios_prohibidos_count": params.horarios_prohibidos.len(),
    });

    match crate::algorithm::ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            let mut out: Vec<serde_json::Value> = Vec::new();
            // CAMBIO: Retornar TODAS las soluciones (sin límite de .take(20))
            for (sol, total_score) in soluciones.into_iter() {
                let mut secciones_json: Vec<serde_json::Value> = Vec::new();
                for (s, prio) in sol.into_iter() {
                    secciones_json.push(json!({"seccion": s, "prioridad": prio}));
                }
                out.push(json!({"total_score": total_score, "secciones": secciones_json}));
            }
            HttpResponse::Ok().json(json!({"status": "ok", "debug": debug_info, "soluciones": out}))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"status": "error", "error": format!("{}", e)})),
    }
}

pub async fn rutacritica_run_dependencies_only_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    use crate::models::RamoDisponible;
    use std::collections::HashMap;

    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    if params.email.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({"error": "email is required"}));
    }

    let initial_map: HashMap<String, RamoDisponible> = HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match crate::algorithm::extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"status": "error", "error": format!("extraction failed: {}", e)})),
    };

    let soluciones = crate::algorithm::get_clique_dependencies_only(&lista_secciones, &ramos_actualizados);

    let mut out: Vec<serde_json::Value> = Vec::new();
    // CAMBIO: Retornar TODAS las soluciones (sin límite de .take(20))
    for (sol, total_score) in soluciones.into_iter() {
        let mut secciones_json: Vec<serde_json::Value> = Vec::new();
        for (s, prio) in sol.into_iter() {
            secciones_json.push(json!({"seccion": s, "prioridad": prio}));
        }
        out.push(json!({"total_score": total_score, "secciones": secciones_json}));
    }
    HttpResponse::Ok().json(json!({"status": "ok", "soluciones": out, "note": "DEPENDENCIES ONLY - NO SCHEDULE CONFLICTS CHECKED"}))
}
