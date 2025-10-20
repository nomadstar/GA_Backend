use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::algorithms::{get_ramo_critico, extract_data, get_clique_max_pond};
use crate::models::Seccion;
use quickshift::api_json::InputParams;

#[derive(Deserialize)]
struct SolveRequest {
    // reuse InputParams structure fields (we accept a superset)
    email: Option<String>,
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

async fn solve_handler(_body: web::Json<serde_json::Value>) -> impl Responder {
    // Run the existing pipeline synchronously
    let (ramos_disponibles, _nombre_excel_malla, malla_leida) = get_ramo_critico();
    let (lista_secciones, ramos_actualizados, oferta_leida) = extract_data(&ramos_disponibles, "MiMalla.xlsx");
    let soluciones = get_clique_max_pond(&lista_secciones, &ramos_actualizados);

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

pub async fn run_server(bind_addr: &str) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/solve", web::post().to(solve_handler))
            .route("/help", web::get().to(help_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
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
        malla: Some("MallaCurricular2020.xlsx".to_string()),
    };

    // Also include a short help message
    let help = json!({
        "description": "Estructura JSON esperada para POST /solve. ramos_pasados debe ser una lista de códigos de ramo (ej: 'CIT3313') tal como aparecen en la fila 'Asignatura' del archivo de Oferta (OfertaAcademica2024.xlsx). Envíe este objeto como body (Content-Type: application/json).",
        "example": example,
        "note_file_reference": "#file:OfertaAcademica2024.xlsx (fila/col 'Asignatura')",
        "malla_choices": ["MallaCurricular2010.xlsx", "MallaCurricular2018.xlsx", "MallaCurricular2020.xlsx"]
    });

    HttpResponse::Ok().json(help)
}
