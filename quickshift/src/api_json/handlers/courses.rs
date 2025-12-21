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

#[derive(Debug, Deserialize)]
pub struct ProfesoresDisponiblesRequest {
    pub malla: String,
    #[serde(default)]
    pub ramos_pasados: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ProfesorCursoDto {
    profesor: String,
    curso_codigo: String,
    curso_nombre: String,
    seccion: String,
    horario: Vec<String>,
    is_cfg: bool,
    is_electivo: bool,
}

#[derive(Debug, Deserialize)]
pub struct CursosDisponiblesRequest {
    pub malla: String,
    #[serde(default)]
    pub ramos_pasados: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CursoDisponibleDto {
    id: i32,
    codigo: String,
    nombre: String,
    semestre: Option<i32>,
    requisitos_ids: Vec<i32>,
    electivo: bool,
    dificultad: Option<f64>,
    is_cfg: bool,
    is_electivo: bool,
}

/// Endpoint que devuelve todos los cursos disponibles para el estudiante,
/// incluyendo cursos de la malla, CFG y electivos de carrera.
pub async fn cursos_disponibles_handler(body: web::Json<CursosDisponiblesRequest>) -> impl Responder {
    let payload = body.into_inner();
    
    // 1. Resolver paths de archivos
    let (malla_pathbuf, oferta_pathbuf, porcentajes_pathbuf) = match resolve_datafile_paths(&payload.malla) {
        Ok(paths) => paths,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("Failed to resolve paths: {}", e)})),
    };
    
    let malla_str = malla_pathbuf.to_string_lossy().to_string();
    let oferta_str = oferta_pathbuf.to_string_lossy().to_string();
    let porcentajes_str = porcentajes_pathbuf.to_string_lossy().to_string();
    
    // 2. Cargar malla
    let ramos_disponibles: HashMap<String, RamoDisponible> = if malla_str.to_uppercase().contains("MC") {
        match leer_mc_con_porcentajes_optimizado(&malla_str, &porcentajes_str) {
            Ok(m) => m,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read malla: {}", e)})),
        }
    } else {
        match leer_malla_con_porcentajes_optimizado(&malla_str, &porcentajes_str) {
            Ok(m) => m,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read malla: {}", e)})),
        }
    };
    
    // 3. Cargar oferta académica
    let mut lista_secciones = match crate::excel::leer_oferta_academica_excel(&oferta_str) {
        Ok(secs) => secs,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read oferta: {}", e)})),
    };
    
    // 4. Cargar CFG si existe
    if let Some(cfg_pathbuf) = crate::excel::latest_file_for_keywords(&["cfg"]) {
        if let Some(cfg_str) = cfg_pathbuf.to_str() {
            if let Ok(cfg_secs) = crate::excel::leer_oferta_academica_excel(cfg_str) {
                for mut s in cfg_secs.into_iter() {
                    let name_norm = normalize_name(&s.nombre);
                    if name_norm == normalize_name("Inglés I") || name_norm == normalize_name("Ingles I") {
                        s.nombre = "Inglés 1".to_string();
                        s.is_cfg = false;
                    } else {
                        s.is_cfg = true;
                    }
                    lista_secciones.push(s);
                }
            }
        }
    }
    
    // 5. Preparar datos para filtrado
    let passed_set: HashSet<String> = payload.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    let passed_names_normalized: HashSet<String> = payload.ramos_pasados
        .iter()
        .map(|s| normalize_name(s))
        .collect();
    
    // Contar CFGs aprobados (máximo 4 permitidos en total)
    let cfgs_aprobados = payload.ramos_pasados.iter()
        .filter(|r| r.to_uppercase().starts_with("CFG"))
        .count();
    let mostrar_cfgs = cfgs_aprobados < 4;
    
    // Contar electivos aprobados
    let codigos_malla: HashSet<String> = ramos_disponibles
        .values()
        .map(|r| r.codigo.to_uppercase())
        .collect();
    let nombres_malla: HashSet<String> = ramos_disponibles
        .values()
        .map(|r| normalize_name(&r.nombre))
        .collect();
    
    let electivos_aprobados = payload.ramos_pasados.iter()
        .filter(|code| {
            let code_upper = code.to_uppercase();
            if code_upper.starts_with("CFG") {
                return false;
            }
            !codigos_malla.contains(&code_upper) && !nombres_malla.contains(&normalize_name(code))
        })
        .count();
    let max_electivos = 3usize;
    let mostrar_electivos = electivos_aprobados < max_electivos;
    
    // Calcular max_sem basado en ramos aprobados
    let mut max_sem = 0;
    for code in &payload.ramos_pasados {
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == code.to_uppercase()) {
            if let Some(s) = r.semestre {
                max_sem = max_sem.max(s);
            }
        }
    }
    let max_sem = max_sem + 2;
    
    // Calcular IDs de ramos aprobados
    let mut aprobados_ids: HashSet<i32> = HashSet::new();
    for ramo in ramos_disponibles.values() {
        if passed_set.contains(&ramo.codigo.to_uppercase()) {
            aprobados_ids.insert(ramo.id);
        }
    }
    
    // 6. Construir lista de cursos disponibles
    let mut cursos_result: Vec<CursoDisponibleDto> = Vec::new();
    let mut cursos_vistos: HashSet<String> = HashSet::new();
    
    // 6a. Agregar cursos de la malla que cumplan prerequisitos
    for ramo in ramos_disponibles.values() {
        // Excluir ramos ya aprobados
        if passed_set.contains(&ramo.codigo.to_uppercase()) || 
           passed_names_normalized.contains(&normalize_name(&ramo.nombre)) {
            continue;
        }
        
        // Verificar semestre
        if let Some(sem) = ramo.semestre {
            if sem > max_sem {
                continue;
            }
        }
        
        // Verificar prerequisitos
        if !ramo.requisitos_ids.iter().all(|req_id| *req_id <= 0 || aprobados_ids.contains(req_id)) {
            continue;
        }
        
        // Verificar que exista en la oferta académica
        let existe_en_oferta = lista_secciones.iter().any(|sec| {
            sec.codigo.to_uppercase() == ramo.codigo.to_uppercase() ||
            normalize_name(&sec.nombre) == normalize_name(&ramo.nombre)
        });
        
        if !existe_en_oferta {
            continue;
        }
        
        let key = ramo.codigo.to_uppercase();
        if cursos_vistos.contains(&key) {
            continue;
        }
        cursos_vistos.insert(key);
        
        cursos_result.push(CursoDisponibleDto {
            id: ramo.id,
            codigo: ramo.codigo.clone(),
            nombre: ramo.nombre.clone(),
            semestre: ramo.semestre,
            requisitos_ids: ramo.requisitos_ids.clone(),
            electivo: ramo.electivo,
            dificultad: ramo.dificultad,
            is_cfg: false,
            is_electivo: false,
        });
    }
    
    // 6b. Agregar CFGs disponibles
    if mostrar_cfgs {
        let mut cfg_id = 1000; // IDs especiales para CFGs
        for sec in lista_secciones.iter().filter(|s| s.is_cfg) {
            // Excluir CFGs ya aprobados
            if passed_set.contains(&sec.codigo.to_uppercase()) || 
               passed_names_normalized.contains(&normalize_name(&sec.nombre)) {
                continue;
            }
            
            let key = sec.codigo.to_uppercase();
            if cursos_vistos.contains(&key) {
                continue;
            }
            cursos_vistos.insert(key);
            
            cursos_result.push(CursoDisponibleDto {
                id: cfg_id,
                codigo: sec.codigo.clone(),
                nombre: sec.nombre.clone(),
                semestre: None,
                requisitos_ids: vec![],
                electivo: false,
                dificultad: None,
                is_cfg: true,
                is_electivo: false,
            });
            cfg_id += 1;
        }
    }
    
    // 6c. Agregar electivos disponibles
    if mostrar_electivos {
        let mut electivo_id = 2000; // IDs especiales para electivos
        for sec in lista_secciones.iter() {
            // Saltar CFGs (ya procesados)
            if sec.is_cfg {
                continue;
            }
            
            // Verificar si NO está en la malla (es electivo)
            let en_malla = ramos_disponibles.values().any(|r| {
                r.codigo.to_uppercase() == sec.codigo.to_uppercase() ||
                normalize_name(&r.nombre) == normalize_name(&sec.nombre)
            });
            
            if en_malla {
                continue;
            }
            
            // Excluir electivos ya aprobados
            if passed_set.contains(&sec.codigo.to_uppercase()) || 
               passed_names_normalized.contains(&normalize_name(&sec.nombre)) {
                continue;
            }
            
            let key = sec.codigo.to_uppercase();
            if cursos_vistos.contains(&key) {
                continue;
            }
            cursos_vistos.insert(key);
            
            cursos_result.push(CursoDisponibleDto {
                id: electivo_id,
                codigo: sec.codigo.clone(),
                nombre: sec.nombre.clone(),
                semestre: None,
                requisitos_ids: vec![],
                electivo: true,
                dificultad: None,
                is_cfg: false,
                is_electivo: true,
            });
            electivo_id += 1;
        }
    }
    
    // 7. Ordenar: primero por semestre (malla), luego CFGs, luego electivos
    cursos_result.sort_by(|a, b| {
        // Primero ordenar por tipo: malla < cfg < electivo
        let type_a = if a.is_cfg { 1 } else if a.is_electivo { 2 } else { 0 };
        let type_b = if b.is_cfg { 1 } else if b.is_electivo { 2 } else { 0 };
        
        type_a.cmp(&type_b)
            .then(a.semestre.unwrap_or(99).cmp(&b.semestre.unwrap_or(99)))
            .then(a.codigo.cmp(&b.codigo))
    });
    
    HttpResponse::Ok().json(json!({
        "malla": payload.malla,
        "resumen": {
            "cfgs_aprobados": cfgs_aprobados,
            "cfgs_faltantes": 4usize.saturating_sub(cfgs_aprobados),
            "electivos_aprobados": electivos_aprobados,
            "electivos_faltantes": max_electivos.saturating_sub(electivos_aprobados),
            "mostrar_cfgs": mostrar_cfgs,
            "mostrar_electivos": mostrar_electivos,
        },
        "total_cursos": cursos_result.len(),
        "cursos_por_tipo": {
            "malla": cursos_result.iter().filter(|c| !c.is_cfg && !c.is_electivo).count(),
            "cfg": cursos_result.iter().filter(|c| c.is_cfg).count(),
            "electivo": cursos_result.iter().filter(|c| c.is_electivo).count(),
        },
        "cursos": cursos_result,
    }))
}

/// Endpoint que devuelve todos los profesores disponibles para cursos que el estudiante puede tomar,
/// incluyendo CFG y electivos.
pub async fn profesores_disponibles_handler(body: web::Json<ProfesoresDisponiblesRequest>) -> impl Responder {
    let payload = body.into_inner();
    
    // 1. Resolver paths de archivos
    let (malla_pathbuf, oferta_pathbuf, porcentajes_pathbuf) = match resolve_datafile_paths(&payload.malla) {
        Ok(paths) => paths,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("Failed to resolve paths: {}", e)})),
    };
    
    let malla_str = malla_pathbuf.to_string_lossy().to_string();
    let oferta_str = oferta_pathbuf.to_string_lossy().to_string();
    let porcentajes_str = porcentajes_pathbuf.to_string_lossy().to_string();
    
    // 2. Cargar malla
    let ramos_disponibles: HashMap<String, RamoDisponible> = if malla_str.to_uppercase().contains("MC") {
        match leer_mc_con_porcentajes_optimizado(&malla_str, &porcentajes_str) {
            Ok(m) => m,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read malla: {}", e)})),
        }
    } else {
        match leer_malla_con_porcentajes_optimizado(&malla_str, &porcentajes_str) {
            Ok(m) => m,
            Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read malla: {}", e)})),
        }
    };
    
    // 3. Cargar oferta académica
    let mut lista_secciones = match crate::excel::leer_oferta_academica_excel(&oferta_str) {
        Ok(secs) => secs,
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to read oferta: {}", e)})),
    };
    
    // 4. Cargar CFG si existe
    if let Some(cfg_pathbuf) = crate::excel::latest_file_for_keywords(&["cfg"]) {
        if let Some(cfg_str) = cfg_pathbuf.to_str() {
            if let Ok(cfg_secs) = crate::excel::leer_oferta_academica_excel(cfg_str) {
                for mut s in cfg_secs.into_iter() {
                    let name_norm = normalize_name(&s.nombre);
                    if name_norm == normalize_name("Inglés I") || name_norm == normalize_name("Ingles I") {
                        s.nombre = "Inglés 1".to_string();
                        s.is_cfg = false;
                    } else {
                        s.is_cfg = true;
                    }
                    lista_secciones.push(s);
                }
            }
        }
    }
    
    // 5. Preparar datos para filtrado
    let passed_set: HashSet<String> = payload.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    // También normalizar nombres de ramos aprobados para comparar
    let passed_names_normalized: HashSet<String> = payload.ramos_pasados
        .iter()
        .map(|s| normalize_name(s))
        .collect();
    
    // Contar CFGs aprobados (máximo 4 permitidos en total)
    let cfgs_aprobados = payload.ramos_pasados.iter()
        .filter(|r| r.to_uppercase().starts_with("CFG"))
        .count();
    let mostrar_cfgs = cfgs_aprobados < 4;
    
    // Contar electivos aprobados (máximo 2-3 permitidos)
    // Electivos son cursos que NO están en la malla y NO son CFG
    let codigos_malla: HashSet<String> = ramos_disponibles
        .values()
        .map(|r| r.codigo.to_uppercase())
        .collect();
    let nombres_malla: HashSet<String> = ramos_disponibles
        .values()
        .map(|r| normalize_name(&r.nombre))
        .collect();
    
    let electivos_aprobados = payload.ramos_pasados.iter()
        .filter(|code| {
            let code_upper = code.to_uppercase();
            // No es CFG
            if code_upper.starts_with("CFG") {
                return false;
            }
            // No está en la malla = es electivo
            !codigos_malla.contains(&code_upper) && !nombres_malla.contains(&normalize_name(code))
        })
        .count();
    let max_electivos = 3usize; // Asumimos máximo 3 electivos requeridos
    let mostrar_electivos = electivos_aprobados < max_electivos;
    
    // Calcular max_sem basado en ramos aprobados
    let mut max_sem = 0;
    for code in &payload.ramos_pasados {
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == code.to_uppercase()) {
            if let Some(s) = r.semestre {
                max_sem = max_sem.max(s);
            }
        }
    }
    let max_sem = max_sem + 2; // Permitir hasta 2 semestres adelante
    
    // Calcular IDs de ramos aprobados para verificar prerequisitos
    let mut aprobados_ids: HashSet<i32> = HashSet::new();
    for ramo in ramos_disponibles.values() {
        if passed_set.contains(&ramo.codigo.to_uppercase()) {
            aprobados_ids.insert(ramo.id);
        }
    }
    
    // 6. Filtrar secciones y extraer profesores
    let mut profesores_result: Vec<ProfesorCursoDto> = Vec::new();
    
    for sec in lista_secciones.iter() {
        // Excluir ramos ya aprobados (por código o nombre)
        if passed_set.contains(&sec.codigo.to_uppercase()) || 
           passed_names_normalized.contains(&normalize_name(&sec.nombre)) {
            continue;
        }
        
        // Si es CFG: mostrar solo si aún no aprobó los 4 CFGs requeridos
        if sec.is_cfg {
            if !mostrar_cfgs {
                continue;
            }
            // CFG válido, agregar profesor
            if !sec.profesor.trim().is_empty() {
                profesores_result.push(ProfesorCursoDto {
                    profesor: sec.profesor.clone(),
                    curso_codigo: sec.codigo.clone(),
                    curso_nombre: sec.nombre.clone(),
                    seccion: sec.seccion.clone(),
                    horario: sec.horario.clone(),
                    is_cfg: true,
                    is_electivo: false,
                });
            }
            continue;
        }
        
        // Verificar si el curso está en la malla
        let ramo_en_malla = ramos_disponibles.values().find(|r| {
            r.codigo.to_uppercase() == sec.codigo.to_uppercase() ||
            normalize_name(&r.nombre) == normalize_name(&sec.nombre)
        });
        
        match ramo_en_malla {
            Some(ramo) => {
                // Curso está en la malla - verificar semestre y prerequisitos
                
                // Verificar semestre
                if let Some(sem) = ramo.semestre {
                    if sem > max_sem {
                        continue;
                    }
                }
                
                // Verificar prerequisitos
                if !ramo.requisitos_ids.iter().all(|req_id| *req_id <= 0 || aprobados_ids.contains(req_id)) {
                    continue;
                }
                
                // Curso de malla válido, agregar profesor
                if !sec.profesor.trim().is_empty() {
                    profesores_result.push(ProfesorCursoDto {
                        profesor: sec.profesor.clone(),
                        curso_codigo: sec.codigo.clone(),
                        curso_nombre: sec.nombre.clone(),
                        seccion: sec.seccion.clone(),
                        horario: sec.horario.clone(),
                        is_cfg: false,
                        is_electivo: false,
                    });
                }
            }
            None => {
                // Curso NO está en la malla = es ELECTIVO
                // Mostrar solo si aún no completó todos los electivos requeridos
                if !mostrar_electivos {
                    continue;
                }
                
                // Electivo válido, agregar profesor
                if !sec.profesor.trim().is_empty() {
                    profesores_result.push(ProfesorCursoDto {
                        profesor: sec.profesor.clone(),
                        curso_codigo: sec.codigo.clone(),
                        curso_nombre: sec.nombre.clone(),
                        seccion: sec.seccion.clone(),
                        horario: sec.horario.clone(),
                        is_cfg: false,
                        is_electivo: true,
                    });
                }
            }
        }
    }
    
    // 8. Ordenar por nombre de profesor
    profesores_result.sort_by(|a, b| a.profesor.cmp(&b.profesor));
    
    // 9. Agrupar por profesor
    let mut profesores_map: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
    for p in profesores_result {
        let entry = profesores_map.entry(p.profesor.clone()).or_insert_with(Vec::new);
        entry.push(json!({
            "curso_codigo": p.curso_codigo,
            "curso_nombre": p.curso_nombre,
            "seccion": p.seccion,
            "horario": p.horario,
            "is_cfg": p.is_cfg,
            "is_electivo": p.is_electivo,
        }));
    }
    
    // 10. Convertir a array ordenado
    let mut result_array: Vec<serde_json::Value> = Vec::new();
    let mut sorted_profs: Vec<String> = profesores_map.keys().cloned().collect();
    sorted_profs.sort();
    
    for prof in sorted_profs {
        let cursos = profesores_map.get(&prof).unwrap();
        result_array.push(json!({
            "profesor": prof,
            "cursos": cursos,
            "total_secciones": cursos.len(),
        }));
    }
    
    // Contar profesores por tipo
    let total_cfg = profesores_map.values()
        .flat_map(|cursos| cursos.iter())
        .filter(|c| c.get("is_cfg").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let total_electivo = profesores_map.values()
        .flat_map(|cursos| cursos.iter())
        .filter(|c| c.get("is_electivo").and_then(|v| v.as_bool()).unwrap_or(false))
        .count();
    let total_malla = profesores_map.values()
        .flat_map(|cursos| cursos.iter())
        .filter(|c| {
            !c.get("is_cfg").and_then(|v| v.as_bool()).unwrap_or(false) &&
            !c.get("is_electivo").and_then(|v| v.as_bool()).unwrap_or(false)
        })
        .count();
    
    HttpResponse::Ok().json(json!({
        "malla": payload.malla,
        "resumen": {
            "cfgs_aprobados": cfgs_aprobados,
            "cfgs_faltantes": 4usize.saturating_sub(cfgs_aprobados),
            "electivos_aprobados": electivos_aprobados,
            "electivos_faltantes": max_electivos.saturating_sub(electivos_aprobados),
            "mostrar_cfgs": mostrar_cfgs,
            "mostrar_electivos": mostrar_electivos,
        },
        "total_profesores": result_array.len(),
        "secciones_por_tipo": {
            "malla": total_malla,
            "cfg": total_cfg,
            "electivo": total_electivo,
        },
        "profesores": result_array,
    }))
}

