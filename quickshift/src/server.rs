use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::algorithm::{extract_data, get_clique_with_user_prefs, list_datafiles, summarize_datafiles};
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
    // Use requested malla and optional sheet from params when extracting data
    let initial_map: std::collections::HashMap<String, crate::models::RamoDisponible> = std::collections::HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("extract failed: {}", e)})),
    };
    let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol, score) in soluciones.iter().take(10) {
        let secs: Vec<Seccion> = sol.iter().map(|(s, _)| s.clone()).collect();
        soluciones_serial.push(SolutionEntry { total_score: *score, secciones: secs });
    }

    // Contamos como leídos: malla + oferta si extract_data fue exitoso
    let documentos = 2usize;

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
            .route("/datafiles/debug/pa-names", web::get().to(debug_pa_names_handler))
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
    // Accept `malla` either as plain filename or with an inline sheet in brackets,
    // e.g. `MiMalla.xlsx[Malla 2020]` or `MiMalla.xlsx[&sheet=Malla 2020]`.
    let raw_malla = match qm.get("malla").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(m) => m,
        None => return HttpResponse::BadRequest().json(json!({"error": "malla query parameter is required"})),
    };

    // Extract optional sheet if provided inline in `malla` using brackets.
    let mut malla = raw_malla.clone();
    let mut sheet_from_brackets: Option<String> = None;
    if let Some(start) = raw_malla.find('[') {
        if let Some(end) = raw_malla.rfind(']') {
            if end > start {
                let inner = raw_malla[start+1..end].trim();
                if !inner.is_empty() {
                    // support forms: "sheet=Name" or "&sheet=Name" or just "Name"
                    let inner = inner.trim_start_matches('&');
                    let sheet_val = if inner.to_lowercase().starts_with("sheet=") {
                        inner[6..].trim().to_string()
                    } else {
                        inner.to_string()
                    };
                    if !sheet_val.is_empty() {
                        sheet_from_brackets = Some(sheet_val);
                    }
                }
                // remove the bracketed part from malla
                malla = raw_malla[..start].trim().to_string();
            }
        }
    }

    // Optional 'sheet' query parameter lets client request a specific internal sheet
    let sheet_qparam: Option<String> = qm.get("sheet").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });
    // precedence: explicit ?sheet=... overrides inline bracket; otherwise use inline
    let sheet_opt: Option<String> = match (sheet_qparam, sheet_from_brackets) {
        (Some(q), _) => Some(q),
        (None, Some(b)) => Some(b),
        _ => None,
    };

    // Ensure the requested malla exists among available datafiles to avoid
    // accidental fallthrough or fuzzy resolution returning a different file.
    if let Ok((available_mallas, _ofertas, _porc)) = list_datafiles() {
        if !available_mallas.iter().any(|name| name == &malla) {
            return HttpResponse::BadRequest().json(json!({"error": format!("malla '{}' not found; available: {:?}", malla, available_mallas)}));
        }
    }

    // Resolve paths via excel module (so only excel reads src/datafiles)
    match summarize_datafiles(&malla, sheet_opt.as_deref()) {
        Ok((malla_path, oferta_path, porcent_path, malla_map, oferta, porcent, porcent_names)) => {
            // Preparar resúmenes para enviar. Por defecto retornamos muestras (para evitar payloads enormes).
            // Si el cliente solicita `full=true` en la query, devolvemos todas las filas.
            let full = qm.get("full").map(|s| { let sl = s.to_lowercase(); sl == "1" || sl == "true" }).unwrap_or(false);

            let malla_sample: Vec<serde_json::Value> = if full {
                malla_map.iter().map(|(code, ramo)| json!({"codigo": code, "nombre": ramo.nombre, "numb_correlativo": ramo.numb_correlativo, "dificultad": ramo.dificultad, "codigo_ref": ramo.codigo_ref})).collect()
            } else {
                malla_map.iter().take(200).map(|(code, ramo)| json!({"codigo": code, "nombre": ramo.nombre, "numb_correlativo": ramo.numb_correlativo, "dificultad": ramo.dificultad, "codigo_ref": ramo.codigo_ref})).collect()
            };

            // Build oferta_sample without consuming `oferta` so we can reuse it
            // later to compute merged mappings (malla <-> oferta <-> porcentajes).
            let oferta_sample: Vec<serde_json::Value> = if full {
                oferta.iter().map(|s| json!(s)).collect()
            } else {
                oferta.iter().take(200).map(|s| json!(s)).collect()
            };

            let mut porcent_sample: Vec<serde_json::Value> = Vec::new();
            if full {
                for (k, v) in porcent.iter() {
                    porcent_sample.push(json!({"codigo": k, "porcentaje": v.0, "total": v.1}));
                }
            } else {
                for (k, v) in porcent.iter().take(200) {
                    porcent_sample.push(json!({"codigo": k, "porcentaje": v.0, "total": v.1}));
                }
            }

            // Construir un sample combinado (malla <-> oferta <-> porcentajes)
            // usando el helper del módulo `algorithm`. Limitamos la salida a 200
            // filas para evitar payloads enormes en respuestas por defecto.
            let merged_full = crate::algorithm::merge_malla_oferta_porcentajes(&malla_map, &oferta, &porcent, &porcent_names);
            let merged_sample: Vec<serde_json::Value> = if full {
                merged_full.into_iter().collect()
            } else {
                merged_full.into_iter().take(200).collect()
            };

            // Intentar listar hojas internas de la malla y de la oferta (si los workbooks contienen varias tablas)
            let malla_sheets = match crate::excel::listar_hojas_malla(&malla_path) {
                Ok(s) => s,
                Err(_) => Vec::new(),
            };
            let oferta_sheets = match crate::excel::listar_hojas_malla(&oferta_path) {
                Ok(s) => s,
                Err(_) => Vec::new(),
            };
            HttpResponse::Ok().json(json!({
                "malla_path": malla_path.to_string_lossy(),
                "oferta_path": oferta_path.to_string_lossy(),
                "porcent_path": porcent_path.to_string_lossy(),
                "malla_sheets": malla_sheets,
                "oferta_sheets": oferta_sheets,
                "malla_sample": malla_sample,
                "oferta_sample": oferta_sample,
                "porcent_sample": porcent_sample
                ,"merged_sample": merged_sample
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
        sheet: None,
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
    // Ejecutar pipeline usando la malla y sheet del input params
    let initial_map: std::collections::HashMap<String, crate::models::RamoDisponible> = std::collections::HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("extract failed: {}", e)})),
    };
    let soluciones = get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    let mut soluciones_serial: Vec<SolutionEntry> = Vec::new();
    for (sol, score) in soluciones.iter().take(10) {
        let secs: Vec<Seccion> = sol.iter().map(|(s, _)| s.clone()).collect();
        soluciones_serial.push(SolutionEntry { total_score: *score, secciones: secs });
    }

    // Contamos como leídos: malla + oferta si extract_data fue exitoso
    let documentos = 2usize;

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
        sheet: None,
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

/// DEBUG: GET /datafiles/debug/pa-names
/// Muestra un sample del índice de nombres normalizados extraídos del PA para diagnóstico
async fn debug_pa_names_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let qm = query.into_inner();
    let porcent_file = match qm.get("porcent").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(f) => f,
        None => "src/datafiles/PA2025-1.xlsx".to_string(),
    };

    match crate::excel::leer_porcentajes_aprobados_con_nombres(&porcent_file) {
        Ok((_porcent_map, porcent_names)) => {
            // Show first 50 entries from the name index
            let mut sample: Vec<serde_json::Value> = Vec::new();
            for (norm_name, (codigo, pct, total, es_electivo)) in porcent_names.iter().take(50) {
                sample.push(json!({
                    "normalized_name": norm_name,
                    "codigo": codigo,
                    "porcentaje": pct,
                    "total": total,
                    "es_electivo": es_electivo
                }));
            }
            HttpResponse::Ok().json(json!({"total_names": porcent_names.len(), "sample": sample}))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to read PA: {}", e)})),
    }
}
