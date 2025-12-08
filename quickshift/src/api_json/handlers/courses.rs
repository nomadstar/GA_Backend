use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};

use crate::excel::{
    leer_malla_con_porcentajes_optimizado,
    leer_mc_con_porcentajes_optimizado,
    normalize_name,
    resolve_datafile_paths,
};
use crate::models::RamoDisponible;

#[derive(Debug, Serialize, Clone)]
struct CursoDto {
    id: i32,
    nombre: String,
    codigo: String,
    semestre: Option<i32>,
    requisitos_ids: Vec<i32>,
    electivo: bool,
    dificultad: Option<f64>,
    numb_correlativo: i32,
    critico: bool,
}

#[derive(Debug, Deserialize)]
pub struct CursosRecomendadosRequest {
    pub malla_id: String,
    #[serde(default)]
    pub ramos_aprobados: Vec<String>,
    #[serde(default)]
    pub sheet: Option<String>,
}

fn ramo_to_dto(r: &RamoDisponible) -> CursoDto {
    CursoDto {
        id: r.id,
        nombre: r.nombre.clone(),
        codigo: r.codigo.clone(),
        semestre: r.semestre,
        requisitos_ids: r.requisitos_ids.clone(),
        electivo: r.electivo,
        dificultad: r.dificultad,
        numb_correlativo: r.numb_correlativo,
        critico: r.critico,
    }
}

fn load_malla_map(malla_id: &str, _sheet: Option<String>) -> Result<HashMap<String, RamoDisponible>, String> {
    let (malla_path, _oferta_path, porcent_path) = resolve_datafile_paths(malla_id)
        .map_err(|e| format!("failed to resolve malla '{}': {}", malla_id, e))?;

    let malla_path_str = malla_path
        .to_str()
        .ok_or_else(|| "invalid UTF-8 in malla path".to_string())?;
    let porcent_path_str = porcent_path
        .to_str()
        .ok_or_else(|| "invalid UTF-8 in porcent path".to_string())?;

    let malla_lower = malla_path_str.to_lowercase();
    let is_mc = malla_lower.contains("mc");

    let res = if is_mc {
        leer_mc_con_porcentajes_optimizado(malla_path_str, porcent_path_str)
    } else {
        leer_malla_con_porcentajes_optimizado(malla_path_str, porcent_path_str)
    };

    res.map_err(|e| format!("failed to read malla '{}': {}", malla_path_str, e))
}

fn sort_cursos(cursos: &mut Vec<CursoDto>) {
    cursos.sort_by(|a, b| {
        let sa = a.semestre.unwrap_or(i32::MAX);
        let sb = b.semestre.unwrap_or(i32::MAX);
        sa.cmp(&sb)
            .then(a.numb_correlativo.cmp(&b.numb_correlativo))
            .then(a.id.cmp(&b.id))
    });
}

fn prerequisitos_cumplidos(ramo: &RamoDisponible, aprobados_ids: &HashSet<i32>) -> bool {
    ramo.requisitos_ids
        .iter()
        .all(|req_id| *req_id <= 0 || aprobados_ids.contains(req_id))
}

fn elegibles_desde_malla(
    map: &HashMap<String, RamoDisponible>,
    aprobados_raw: &[String],
) -> Vec<CursoDto> {
    let aprobados_limpios: Vec<String> = aprobados_raw
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let aprobados_codes_upper: HashSet<String> =
        aprobados_limpios.iter().map(|s| s.to_uppercase()).collect();
    let aprobados_norm: HashSet<String> =
        aprobados_limpios.iter().map(|s| normalize_name(s)).collect();

    let mut aprobados_ids: HashSet<i32> = HashSet::new();
    for ramo in map.values() {
        let code_upper = ramo.codigo.to_uppercase();
        let name_norm = normalize_name(&ramo.nombre);
        if (!code_upper.is_empty() && aprobados_codes_upper.contains(&code_upper))
            || aprobados_norm.contains(&name_norm)
        {
            aprobados_ids.insert(ramo.id);
        }
    }

    let mut elegibles: Vec<CursoDto> = map
        .values()
        .filter(|r| {
            let code_upper = r.codigo.to_uppercase();
            !aprobados_ids.contains(&r.id)
                && !(!code_upper.is_empty() && aprobados_codes_upper.contains(&code_upper))
                && prerequisitos_cumplidos(r, &aprobados_ids)
        })
        .map(ramo_to_dto)
        .collect();

    sort_cursos(&mut elegibles);
    elegibles
}

pub async fn cursos_por_semestre_handler(
    path: web::Path<(String, i32)>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let (malla_id, semestre) = path.into_inner();
    let sheet = query
        .get("sheet")
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });

    match load_malla_map(&malla_id, sheet) {
        Ok(map) => {
            let mut cursos: Vec<CursoDto> = map
                .values()
                .filter(|r| r.semestre == Some(semestre))
                .map(ramo_to_dto)
                .collect();
            sort_cursos(&mut cursos);
            HttpResponse::Ok().json(json!({
                "malla": malla_id,
                "semestre": semestre,
                "cursos": cursos
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

pub async fn cursos_todos_handler(
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let malla_id = path.into_inner();
    let sheet = query
        .get("sheet")
        .and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });

    match load_malla_map(&malla_id, sheet) {
        Ok(map) => {
            let mut cursos: Vec<CursoDto> = map.values().map(ramo_to_dto).collect();
            sort_cursos(&mut cursos);
            HttpResponse::Ok().json(json!({
                "malla": malla_id,
                "cursos": cursos
            }))
        }
        Err(e) => HttpResponse::BadRequest().json(json!({ "error": e })),
    }
}

pub async fn cursos_recomendados_handler(body: web::Json<CursosRecomendadosRequest>) -> impl Responder {
    let payload = body.into_inner();
    let sheet = payload.sheet.clone();

    let map = match load_malla_map(&payload.malla_id, sheet) {
        Ok(m) => m,
        Err(e) => return HttpResponse::BadRequest().json(json!({ "error": e })),
    };

    let elegibles = elegibles_desde_malla(&map, &payload.ramos_aprobados);

    HttpResponse::Ok().json(json!({
        "malla": payload.malla_id,
        "total_elegibles": elegibles.len(),
        "cursos": elegibles
    }))
}

