use actix_web::{HttpResponse, Responder};
use serde_json::json;
use crate::api_json::InputParams;

pub async fn help_handler() -> impl Responder {
    let example = InputParams {
        email: "alumno@ejemplo.cl".to_string(),
        ramos_pasados: vec!["CIT3313".to_string(), "CIT3211".to_string()],
        ramos_prioritarios: vec!["CIT3313".to_string(), "CIT3413".to_string()],
        horarios_preferidos: vec!["08:00-10:00".to_string(), "14:00-16:00".to_string()],
        horarios_prohibidos: Vec::new(),
        malla: "malla.xlsx".to_string(),
        anio: None,
        sheet: None,
        ranking: None,
        student_ranking: None,
        filtros: None,
        optimizations: Vec::new(),
    };

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
