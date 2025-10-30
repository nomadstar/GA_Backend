// Port literal del script Python `RutaCritica/extract_data.py` a Rust.
//
// Este módulo implementa las mismas funciones auxiliares que el script
// original: equivalencia, counters, appendElectivos, secciones_cfg y
// extract_data. Está escrito de manera relativamente literal para facilitar
// la revisión y posteriores refactorizaciones.

use std::collections::HashMap;
use std::error::Error;

use crate::models::{Seccion, RamoDisponible};
use crate::excel; // usamos la API de alto nivel de excel

/// extract_data simplificado: delega en `excel` para obtener rutas y lectura.
/// Devuelve (secciones, ramos_disponibles) o error.
/// 
/// Flujo:
/// 1. Resuelve rutas de archivos (malla, oferta, porcentajes)
/// 2. Lee Malla2020 enriquecida con datos de porcentajes (nombre → código + dificultad)
/// 3. Lee oferta académica para obtener secciones
pub fn extract_data(
    _ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
    _sheet: Option<&str>,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    // Resolver rutas de malla, oferta y porcentajes en DATAFILES_DIR
    let (malla_path, oferta_path, porcent_path) = excel::resolve_datafile_paths(nombre_excel_malla)?;

    // 1) Leer Malla2020 enriquecida con porcentajes (Nombre → Código + Dificultad)
    // Esto reemplaza la lectura manual de malla + enriquecimiento manual
    let malla_str = malla_path.to_str().ok_or("ruta malla no UTF-8")?;
    let porcent_str = porcent_path.to_str().ok_or("ruta porcent no UTF-8")?;
    
    let ramos_disponibles = match excel::leer_malla_con_porcentajes(malla_str, porcent_str) {
        Ok(ramos_map) => {
            eprintln!("DEBUG: Malla2020 enriquecida con porcentajes: {} ramos cargados", ramos_map.len());
            ramos_map
        }
        Err(e) => {
            eprintln!("WARN: No se pudo enriquecer Malla2020 con porcentajes: {}. Intentando con lectura alternativa...", e);
            // Fallback: leer malla sin enriquecimiento
            match excel::leer_malla_excel_with_sheet(malla_str, Some("Malla2020")) {
                Ok(ramos_map) => {
                    eprintln!("DEBUG: Malla2020 cargada sin porcentajes: {} ramos", ramos_map.len());
                    ramos_map
                }
                Err(e2) => {
                    return Err(format!("error leyendo malla '{}': {} (fallback también falló: {})", malla_str, e, e2).into());
                }
            }
        }
    };

    // 2) Leer oferta académica -> obtener secciones
    let oferta_str = oferta_path.to_str().ok_or("ruta oferta no UTF-8")?;
    let secciones: Vec<Seccion> = match excel::leer_oferta_academica_excel(oferta_str) {
        Ok(s) => {
            eprintln!("DEBUG: Oferta académica cargada: {} secciones totales", s.len());
            s
        }
        Err(e) => {
            // Fallback tolerante: si falla la oferta, usamos un conjunto vacío
            eprintln!("WARN: no se pudo leer oferta '{}': {}. Usando lista de secciones vacía.", oferta_str, e);
            Vec::new()
        }
    };

    // 3) IMPORTANTE: Filtrar secciones para que solo incluya aquellas que existen en la Malla
    // Esto es crítico porque OA2024 contiene muchos cursos que no están en Malla2020
    let total_secciones = secciones.len();
    let secciones_filtradas: Vec<Seccion> = secciones.into_iter().filter(|sec| {
        let nombre_norm = crate::excel::normalize_name(&sec.nombre);
        // Aceptar si existe en ramos_disponibles (de Malla) O si es electivo
        ramos_disponibles.contains_key(&nombre_norm) || nombre_norm == "electivo profesional"
    }).collect();
    
    eprintln!("DEBUG: Secciones filtradas por Malla2020: {} → {} (quedaron)", 
              total_secciones, secciones_filtradas.len());

    Ok((secciones_filtradas, ramos_disponibles))
}

// get_ramo_critico fue movido a `crate::excel::get_ramo_critico` para
// centralizar todo el acceso a archivos Excel en el módulo `excel`.
