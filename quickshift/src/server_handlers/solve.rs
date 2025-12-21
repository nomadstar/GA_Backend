use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde_json::json;
use crate::api_json::InputParams;
use crate::models::Seccion;
use std::sync::OnceLock;
use std::sync::Arc;
use tokio::sync::Semaphore;
use num_cpus;

#[derive(serde::Deserialize)]
struct SolveRequest {
    _email: Option<String>,
}

#[derive(serde::Serialize)]
struct SolveResponse {
    documentos_leidos: usize,
    soluciones_count: usize,
    soluciones: Vec<SolutionEntry>,
}

#[derive(serde::Serialize)]
struct SolutionEntry {
    total_score: i64,
    secciones: Vec<Seccion>,
}

pub async fn solve_handler(req: HttpRequest, body: web::Json<serde_json::Value>) -> impl Responder {
    // Reuse original logic from server.rs: parse, resolve, spawn_blocking with semaphore.
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    let client_ip = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
    let start = std::time::Instant::now();

    static GLOBAL_SEM: OnceLock<Arc<Semaphore>> = OnceLock::new();
    let sem = GLOBAL_SEM.get_or_init(|| {
        let procs = num_cpus::get();
        Arc::new(Semaphore::new(std::cmp::max(1, procs)))
    }).clone();

    let permit = match sem.clone().acquire_owned().await {
        Ok(p) => p,
        Err(_) => return HttpResponse::InternalServerError().json(json!({"error": "failed to acquire semaphore"})),
    };

    let params_block = params;

    let blocking_handle = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        // USAR LA NUEVA FUNCIÓN 4-FASES CON FILTRAJE CORRECTO
        match crate::algorithm::ruta::ejecutar_ruta_critica_with_params(params_block) {
            Ok(soluciones) => {
                // soluciones es Vec<(Vec<(Seccion, i32)>, i64)>
                // necesitamos extraer lista_secciones y ramos_actualizados para luego serializar
                // Por ahora, solo retornamos soluciones
                Ok(soluciones)
            },
            Err(e) => Err(format!("ruta_critica failed: {}", e)),
        }
    });

    let blocking_result = match blocking_handle.await {
        Ok(res) => res,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("task join error: {}", e)})),
    };

    let soluciones = match blocking_result {
        Ok(v) => v,
        Err(err_msg) => return HttpResponse::InternalServerError().json(json!({"error": err_msg})),
    };

    // Convertir Vec<(Vec<(Seccion, i32)>, i64)> a Vec<SolutionEntry>
    // NO filtrar por available_codes porque las secciones ya fueron validadas por el algoritmo
    // CAMBIO: Retornar TODAS las soluciones (sin límite de .take(20))
    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol_with_prefs, score) in soluciones.iter() {
        // Extraer todas las secciones (ya validadas por el algoritmo)
        let final_secs: Vec<Seccion> = sol_with_prefs.iter()
            .map(|(sec, _pref)| sec.clone())
            .collect();
        
        // Agregar la solución con todas sus secciones
        if !final_secs.is_empty() {
            soluciones_serial.push(SolutionEntry { total_score: *score, secciones: final_secs });
        }
    }

    let documentos = 2usize;

    let resp = SolveResponse {
        documentos_leidos: documentos,
        soluciones_count: soluciones.len(),
        soluciones: soluciones_serial,
    };

    let duration_ms = start.elapsed().as_millis() as i64;

    let req_clone = json_str.clone();
    let resp_ser = match serde_json::to_string(&resp) {
        Ok(s) => s,
        Err(_) => String::from("{}"),
    };
    let resp_clone = resp_ser.clone();
    let ip_clone = client_ip.clone();
    tokio::task::spawn_blocking(move || {
        let _ = crate::analithics::log_query(&req_clone, &resp_clone, duration_ms, &ip_clone);
    });

    HttpResponse::Ok().json(resp)
}

pub async fn solve_get_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let split_list = |s_opt: Option<&String>| -> Vec<String> {
        match s_opt {
            Some(s) if !s.trim().is_empty() => s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect(),
            _ => Vec::new(),
        }
    };

    let qm = query.into_inner();
    let ramos_pasados = split_list(qm.get("ramos_pasados"));
    let ramos_prioritarios = split_list(qm.get("ramos_prioritarios"));
    let horarios_preferidos = split_list(qm.get("horarios_preferidos"));
    let malla = match qm.get("malla").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(m) => m,
        None => return HttpResponse::BadRequest().json(json!({"error": "malla is required in query"})),
    };

    let email = qm.get("email").cloned().unwrap_or_else(|| "".to_string());

        let input = InputParams {
        email,
        ramos_pasados,
        ramos_prioritarios,
        horarios_preferidos,
        horarios_prohibidos: Vec::new(),
        malla,
        sheet: None,
        ranking: None,
        student_ranking: None,
        anio: None,
        filtros: None,
        optimizations: Vec::new(),
    };

    let json_str = match serde_json::to_string(&input) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to serialize input: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to resolve names: {}", e)})),
    };

    // USAR LA NUEVA FUNCIÓN 4-FASES CON FILTRAJE CORRECTO
    let soluciones = match crate::algorithm::ruta::ejecutar_ruta_critica_with_params(params) {
        Ok(sols) => sols,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("ruta_critica failed: {}", e)})),
    };

    // Convertir Vec<(Vec<(Seccion, i32)>, i64)> a Vec<SolutionEntry>
    // NO filtrar por available_codes porque las secciones ya fueron validadas por el algoritmo
    // CAMBIO: Retornar TODAS las soluciones (sin límite de .take(20))
    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol_with_prefs, score) in soluciones.iter() {
        // Extraer todas las secciones (ya validadas por el algoritmo)
        let final_secs: Vec<Seccion> = sol_with_prefs.iter()
            .map(|(sec, _pref)| sec.clone())
            .collect();
        
        // Agregar la solución con todas sus secciones
        if !final_secs.is_empty() {
            soluciones_serial.push(SolutionEntry { total_score: *score, secciones: final_secs });
        }
    }

    let documentos = 2usize;

    let resp = SolveResponse {
        documentos_leidos: documentos,
        soluciones_count: soluciones.len(),
        soluciones: soluciones_serial,
    };

    HttpResponse::Ok().json(resp)
}
