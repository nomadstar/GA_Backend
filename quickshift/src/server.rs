use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::algorithm::{get_ramo_critico, extract_data, get_clique_with_user_prefs, list_datafiles, summarize_datafiles};
use crate::models::Seccion;
use crate::api_json::InputParams;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::Path;

#[derive(Deserialize)]
struct SolveRequest {
    // reuse InputParams structure fields (we accept a superset)
    _email: Option<String>,
}

#[derive(Serialize)]
struct SolveResponse {
    documentos_leidos: usize,
    soluciones_count: usize,
    soluciones: Vec<SolutionEntry>,
}

#[derive(Serialize)]
struct SolutionEntry {
    total_score: i64,
    secciones: Vec<Seccion>,
}

async fn solve_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    // Parse and resolve InputParams from the incoming JSON body (may contain names)
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    // Run the existing pipeline using the resolved params to influence selection
    let (ramos_disponibles, _nombre_excel_malla, malla_leida) = get_ramo_critico();
    let (lista_secciones, ramos_actualizados) = match extract_data(ramos_disponibles.clone(), "MiMalla.xlsx") {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("extract failed: {}", e)})),
    };
    let oferta_leida = true;
    let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol, score) in soluciones.iter().take(10) {
        let secs: Vec<Seccion> = sol.iter().map(|(s, _)| s.clone()).collect();
        soluciones_serial.push(SolutionEntry { total_score: *score, secciones: secs });
    }

    let mut documentos = 0usize;
    if malla_leida { documentos += 1; }
    if oferta_leida { documentos += 1; }

    let resp = SolveResponse {
        documentos_leidos: documentos,
        soluciones_count: soluciones.len(),
        soluciones: soluciones_serial,
    };

    HttpResponse::Ok().json(resp)
}

/// Handler para obtener los mejores caminos desde un JSON de `PathsOutput` o un
/// `file_path` que apunte a un JSON en disco generado por Ruta crítica.
async fn rutacomoda_best_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    // Now this endpoint delegates to the algorithm orchestrator: it expects
    // an `InputParams`-compatible JSON body (same as /rutacritica/run) and
    // returns the best path(s) computed by `ejecutar_ruta_critica_with_params`.
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    match crate::algorithm::ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            // soluciones: Vec<(Vec<(Seccion, i32)>, i64)>
            if soluciones.is_empty() {
                return HttpResponse::Ok().json(json!({"best": []}));
            }

            // Encontrar el score máximo
            let mut max_score: Option<i64> = None;
            for (_sol, score) in soluciones.iter() {
                match max_score {
                    None => max_score = Some(*score),
                    Some(ms) => if *score > ms { max_score = Some(*score); }
                }
            }

            let ms = max_score.unwrap_or(0);
            // Filtrar soluciones que tengan score == ms
            let mut bests: Vec<serde_json::Value> = Vec::new();
            for (sol, score) in soluciones.into_iter() {
                if score == ms {
                    // Mapear solución a lista de códigos
                    let path_codes: Vec<String> = sol.into_iter().map(|(s, _prio)| s.codigo).collect();
                    bests.push(json!({"path": path_codes, "score": score}));
                }
            }

            HttpResponse::Ok().json(json!({"best": bests}))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("algorithm error: {}", e)})),
    }
}

async fn rutacritica_run_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    // Esperamos un JSON con la forma de `InputParams` (o campos equivalentes que se resolverán).
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    // Resolver ramos por nombre si fuera necesario usando la utilidad existente
    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    // Llamar al orquestador público que extrae y ejecuta la ruta crítica con params
    match crate::algorithm::ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            let mut out: Vec<serde_json::Value> = Vec::new();
            for (sol, total_score) in soluciones.into_iter().take(10) {
                let mut secciones_json: Vec<serde_json::Value> = Vec::new();
                for (s, prio) in sol.into_iter() {
                    secciones_json.push(json!({"seccion": s, "prioridad": prio}));
                }
                out.push(json!({"total_score": total_score, "secciones": secciones_json}));
            }
            HttpResponse::Ok().json(json!({"status": "ok", "soluciones": out}))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"status": "error", "error": format!("{}", e)})),
    }
}

/// POST /students
/// Guarda los datos del estudiante en `data/students.json`. Si ya existe un
/// estudiante con el mismo correo, lo sustituye.
async fn save_student_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    // Normalizar y resolver nombres usando la función existente
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let student = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    if student.email.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({"error": "email is required"}));
    }

    // Asegurar directorio
    let data_dir = "data";
    if let Err(e) = create_dir_all(data_dir) {
        return HttpResponse::InternalServerError().json(json!({"error": format!("failed to create data dir: {}", e)}));
    }

    let file_path = format!("{}/students.json", data_dir);
    let mut students: Vec<InputParams> = Vec::new();
    if Path::new(&file_path).exists() {
        match std::fs::read_to_string(&file_path) {
            Ok(contents) if !contents.trim().is_empty() => {
                match serde_json::from_str::<Vec<InputParams>>(&contents) {
                    Ok(mut v) => students.append(&mut v),
                    Err(_) => {
                        // If file exists but is invalid, overwrite it (start fresh)
                        students = Vec::new();
                    }
                }
            }
            _ => { /* empty file or read error -> start fresh */ }
        }
    }

    // Remove existing with same email
    students.retain(|s| s.email.to_lowercase() != student.email.to_lowercase());
    students.push(student);

    // Write back
    match OpenOptions::new().write(true).create(true).truncate(true).open(&file_path) {
        Ok(mut f) => {
            match serde_json::to_string_pretty(&students) {
                Ok(text) => {
                    if let Err(e) = f.write_all(text.as_bytes()) {
                        return HttpResponse::InternalServerError().json(json!({"error": format!("failed to write file: {}", e)}));
                    }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to serialize students: {}", e)})),
            }
        }
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to open file: {}", e)})),
    }

    HttpResponse::Ok().json(json!({"status": "ok", "count": students.len()}))
}

pub async fn run_server(bind_addr: &str) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/solve", web::post().to(solve_handler))
            .route("/solve", web::get().to(solve_get_handler))
                .route("/students", web::post().to(save_student_handler))
            .route("/rutacomoda/best", web::post().to(rutacomoda_best_handler))
            .route("/rutacritica/run", web::post().to(rutacritica_run_handler))
            .route("/datafiles", web::get().to(datafiles_list_handler))
            .route("/datafiles/content", web::get().to(datafiles_content_handler))
            .route("/help", web::get().to(help_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
}

/// GET /datafiles
/// Lista los nombres de archivos MC, OA y PA disponibles en `src/datafiles`.
async fn datafiles_list_handler() -> impl Responder {
    match list_datafiles() {
        Ok((mallas, ofertas, porcentajes)) => HttpResponse::Ok().json(json!({"mallas": mallas, "ofertas": ofertas, "porcentajes": porcentajes})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to list datafiles: {}", e)})),
    }
}

/// GET /datafiles/content?malla=MiMalla.xlsx
/// Devuelve un resumen de los contenidos (primeros elementos) de MALLA, OA y PA
async fn datafiles_content_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let qm = query.into_inner();
    let malla = match qm.get("malla").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(m) => m,
        None => return HttpResponse::BadRequest().json(json!({"error": "malla query parameter is required"})),
    };

    // Optional 'sheet' query parameter lets client request a specific internal sheet
    let sheet_opt: Option<String> = qm.get("sheet").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });

    // Resolve paths via excel module (so only excel reads src/datafiles)
    match summarize_datafiles(&malla, sheet_opt.as_deref()) {
        Ok((malla_path, oferta_path, porcent_path, malla_map, oferta, porcent)) => {
            // Preparar resúmenes para enviar (no volcar todo si es grande)
            let mut malla_sample: Vec<serde_json::Value> = Vec::new();
            for (code, ramo) in malla_map.iter().take(200) {
                malla_sample.push(json!({"codigo": code, "nombre": ramo.nombre, "numb_correlativo": ramo.numb_correlativo, "dificultad": ramo.dificultad, "codigo_ref": ramo.codigo_ref}));
            }

            let oferta_sample: Vec<serde_json::Value> = oferta.into_iter().take(200).map(|s| json!(s)).collect();
            let mut porcent_sample: Vec<serde_json::Value> = Vec::new();
            for (k, v) in porcent.iter().take(200) {
                porcent_sample.push(json!({"codigo": k, "porcentaje": v.0, "total": v.1}));
            }

            // Intentar listar hojas internas de la malla (si el workbook contiene varias tablas)
            let malla_sheets = match crate::excel::listar_hojas_malla(&malla_path) {
                Ok(s) => s,
                Err(_) => Vec::new(),
            };
            HttpResponse::Ok().json(json!({
                "malla_path": malla_path.to_string_lossy(),
                "oferta_path": oferta_path.to_string_lossy(),
                "porcent_path": porcent_path.to_string_lossy(),
                "malla_sheets": malla_sheets,
                "malla_sample": malla_sample,
                "oferta_sample": oferta_sample,
                "porcent_sample": porcent_sample
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(json!({"error": format!("failed to resolve paths for malla '{}': {}", malla, e)})),
    }
}

/// GET /solve handler: acepta parámetros simples en query string.
/// Parámetros esperados (comma-separated lists):
/// - ramos_pasados
/// - ramos_prioritarios
/// - horarios_preferidos
/// - malla
/// - email
async fn solve_get_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    // Helper para convertir 'a,b,c' -> Vec<String>
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
    // malla and tasas are required for the route-critical pipeline.
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
        ranking: None,
        student_ranking: None,
    };

    // Serializar y reutilizar el resolutor existente (esto permitirá usar la
    // resolución por `malla` si se entrega)
    let json_str = match serde_json::to_string(&input) {
        Ok(s) => s,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to serialize input: {}", e)})),
    };

    let params = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(p) => p,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to resolve names: {}", e)})),
    };

    // Ejecutar pipeline
    let (ramos_disponibles, _nombre_excel_malla, malla_leida) = get_ramo_critico();
    let (lista_secciones, ramos_actualizados) = match extract_data(ramos_disponibles.clone(), "MiMalla.xlsx") {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("extract failed: {}", e)})),
    };
    let oferta_leida = true;
    let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol, score) in soluciones.iter().take(10) {
        let secs: Vec<Seccion> = sol.iter().map(|(s, _)| s.clone()).collect();
        soluciones_serial.push(SolutionEntry { total_score: *score, secciones: secs });
    }

    let mut documentos = 0usize;
    if malla_leida { documentos += 1; }
    if oferta_leida { documentos += 1; }

    let resp = SolveResponse {
        documentos_leidos: documentos,
        soluciones_count: soluciones.len(),
        soluciones: soluciones_serial,
    };

    HttpResponse::Ok().json(resp)
}

async fn help_handler() -> impl Responder {
    // Example InputParams to show expected format for POST /solve
    // Use course codes (e.g., "CIT3313") for ramos_pasados. These codes correspond to the
    // values in the 'Asignatura' row/column of the Oferta Academica workbook (see #file:OfertaAcademica2024.xlsx).
    let example = InputParams {
        email: "alumno@ejemplo.cl".to_string(),
        ramos_pasados: vec!["CIT3313".to_string(), "CIT3211".to_string()],
        ramos_prioritarios: vec!["CIT3313".to_string(), "CIT3413".to_string()],
        horarios_preferidos: vec!["08:00-10:00".to_string(), "14:00-16:00".to_string()],
        malla: "MallaCurricular2020.xlsx".to_string(),
        ranking: None,
        student_ranking: None,
    };

    // Also include a short help message
    let help = json!({
        "description": "API para obtener soluciones de horario. POST /solve acepta un JSON complejo (ver 'example') y soporta resolución de nombres usando 'malla'. GET /solve acepta parámetros simples en query (listas separadas por comas).",
        "post_example": example,
        "get_example_query": "/solve?ramos_pasados=CIT3313,CIT3211&ramos_prioritarios=CIT3413&horarios_preferidos=08:00-10:00&malla=MallaCurricular2020.xlsx&email=alumno%40ejemplo.cl",
        "note": "GET es una versión ligera: los parámetros son listas separadas por comas. Para JSON complejo o datos privados use POST con body JSON.",
        "note_file_reference": "#file:OfertaAcademica2024.xlsx (fila/col 'Asignatura')",
        "malla_choices": ["MallaCurricular2010.xlsx", "MallaCurricular2018.xlsx", "MallaCurricular2020.xlsx"]
    });

    HttpResponse::Ok().json(help)
}
