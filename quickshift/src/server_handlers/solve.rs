use actix_web::{web, HttpResponse, Responder, HttpRequest};
use serde_json::json;
use crate::api_json::InputParams;
use crate::algorithm::{extract_data, get_clique_with_user_prefs, select_non_conflicting_sections};
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

    let student_rank_outer = params.student_ranking.unwrap_or(0.5);

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
        let initial_map: std::collections::HashMap<String, crate::models::RamoDisponible> = std::collections::HashMap::new();
        let sheet_opt = params_block.sheet.as_deref();
        let (lista_secciones, ramos_actualizados) = match extract_data(initial_map, &params_block.malla, sheet_opt) {
            Ok((ls, ra)) => (ls, ra),
            Err(e) => return Err(format!("extract failed: {}", e)),
        };
        let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params_block);
        Ok((lista_secciones, ramos_actualizados, soluciones))
    });

    let blocking_result = match blocking_handle.await {
        Ok(res) => res,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("task join error: {}", e)})),
    };

    let (lista_secciones, ramos_actualizados, soluciones) = match blocking_result {
        Ok(v) => v,
        Err(err_msg) => return HttpResponse::InternalServerError().json(json!({"error": err_msg})),
    };

    const DEFAULT_THRESHOLD: f64 = 0.05;
    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    let student_rank = student_rank_outer;
    for (sol, score) in soluciones.iter() {
        let mut prod_reprobar: f64 = 1.0;
        for (s, _prio) in sol.iter() {
            let code_l = s.codigo.to_lowercase();
            if let Some(ramo) = ramos_actualizados.get(&code_l) {
                if let Some(pct_aprob) = ramo.dificultad {
                    let repro = (100.0 - pct_aprob) / 100.0;
                    let repro_eff = repro * (1.0 - student_rank);
                    prod_reprobar *= repro_eff.max(0.0).min(1.0);
                } else {
                    prod_reprobar *= 0.0;
                }
            } else {
                prod_reprobar *= 0.0;
            }
        }

        if prod_reprobar > DEFAULT_THRESHOLD {
            continue;
        }

        if soluciones_serial.len() < 10 {
            // Intentar seleccionar una sección por ramo sin conflicto entre todas las opciones
            // disponibles en `lista_secciones` para los ramos incluidos en `sol`.
            // Construir lista ordenada de claves de ramo (normalize_name) en el mismo orden
            let mut ramo_keys: Vec<String> = Vec::new();
            for (s, _p) in sol.iter() {
                ramo_keys.push(crate::excel::normalize_name(&s.nombre));
            }

            // Construir grupos de candidatos: para cada clave, todas las secciones en lista_secciones
            // cuyo nombre normalizado coincide.
            let mut candidate_groups: Vec<Vec<Seccion>> = Vec::new();
            for rk in ramo_keys.iter() {
                let mut group: Vec<Seccion> = lista_secciones.iter()
                    .filter(|ss| crate::excel::normalize_name(&ss.nombre) == *rk)
                    .cloned()
                    .collect();
                // Si no hay candidatas por nombre, intentar fallback por codigo_box que aparezca en sol
                if group.is_empty() {
                    // Buscar código_box en la solución para este ramo
                    if let Some((first_sec, _)) = sol.iter().find(|(s, _)| crate::excel::normalize_name(&s.nombre) == *rk) {
                        let cb = first_sec.codigo_box.clone();
                        group = lista_secciones.iter().filter(|ss| ss.codigo_box == cb).cloned().collect();
                    }
                }
                candidate_groups.push(group);
            }

            let final_secs: Vec<Seccion> = match select_non_conflicting_sections(&candidate_groups) {
                Some(sel) => sel,
                None => sol.iter().map(|(s, _)| s.clone()).collect(),
            };

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
        malla,
        sheet: None,
        ranking: None,
        student_ranking: None,
            anio: None,
        filtros: None,
    };

    let json_str = match serde_json::to_string(&input) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to serialize input: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to resolve names: {}", e)})),
    };

    let initial_map: std::collections::HashMap<String, crate::models::RamoDisponible> = std::collections::HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("extract failed: {}", e)})),
    };
    let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol, score) in soluciones.iter().take(10) {
        // Para la ruta GET simplificada, aplicamos la misma selección de secciones
        // usando `lista_secciones` como fuente de candidatas.
        let mut ramo_keys: Vec<String> = Vec::new();
        for (s, _p) in sol.iter() {
            ramo_keys.push(crate::excel::normalize_name(&s.nombre));
        }

        let mut candidate_groups: Vec<Vec<Seccion>> = Vec::new();
        for rk in ramo_keys.iter() {
            let mut group: Vec<Seccion> = lista_secciones.iter()
                .filter(|ss| crate::excel::normalize_name(&ss.nombre) == *rk)
                .cloned()
                .collect();
            if group.is_empty() {
                if let Some((first_sec, _)) = sol.iter().find(|(s, _)| crate::excel::normalize_name(&s.nombre) == *rk) {
                    let cb = first_sec.codigo_box.clone();
                    group = lista_secciones.iter().filter(|ss| ss.codigo_box == cb).cloned().collect();
                }
            }
            candidate_groups.push(group);
        }

        let final_secs: Vec<Seccion> = match select_non_conflicting_sections(&candidate_groups) {
            Some(sel) => sel,
            None => sol.iter().map(|(s, _)| s.clone()).collect(),
        };

        soluciones_serial.push(SolutionEntry { total_score: *score, secciones: final_secs });
    }

    let documentos = 2usize;

    let resp = SolveResponse {
        documentos_leidos: documentos,
        soluciones_count: soluciones.len(),
        soluciones: soluciones_serial,
    };

    HttpResponse::Ok().json(resp)
}
