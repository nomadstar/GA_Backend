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
pub fn extract_data(
    mut ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
    sheet: Option<&str>,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    // Resolver rutas de malla, oferta y porcentajes en DATAFILES_DIR
    let (malla_path, oferta_path, porcent_path) = excel::resolve_datafile_paths(nombre_excel_malla)?;

    // 1) Leer malla: debe poblar ramos_disponibles si estaba vacío o actualizarlo.
    // La firma de leer_malla_excel puede variar; adaptarla si es necesario.
    // Asumimos que leer_malla_excel puede aceptar &Path or &str; convertimos a str.
    let malla_str = malla_path.to_str().ok_or("ruta malla no UTF-8")?;
    match excel::leer_malla_excel_with_sheet(malla_str, sheet) {
        Ok(malla_map) => {
            if ramos_disponibles.is_empty() {
                ramos_disponibles = malla_map;
            } else {
                // mezclar/actualizar campos existentes
                for (k, v) in malla_map {
                    ramos_disponibles.entry(k).or_insert(v);
                }
            }
        }
        Err(e) => {
            // si falla la lectura, retornamos error porque la malla es obligatoria para la ruta crítica
            return Err(format!("error leyendo malla '{}': {}", malla_str, e).into());
        }
    }

    // 2) Leer oferta académica -> obtener secciones
    let oferta_str = oferta_path.to_str().ok_or("ruta oferta no UTF-8")?;
    let secciones: Vec<Seccion> = match excel::leer_oferta_academica_excel(oferta_str) {
        Ok(s) => s,
        Err(e) => return Err(format!("error leyendo oferta '{}': {}", oferta_str, e).into()),
    };

    // 3) Leer porcentajes y asignar dificultad a ramos_disponibles cuando corresponda
    let porcent_str = porcent_path.to_str().ok_or("ruta porcent no UTF-8")?;
    if let Ok(map_porcent) = excel::leer_porcentajes_aprobados(porcent_str) {
        // map_porcent: HashMap<String, (f64, f64)> => (porcentaje_aprobados, total)
        for (codigo, ramo) in ramos_disponibles.iter_mut() {
            if let Some((porc, _total)) = map_porcent.get(&codigo.to_string()) {
                ramo.dificultad = Some(*porc);
            }
        }
    } else {
        // Si no hay porcentajes, no bloqueamos — pero se recomienda tenerlos
    }

    Ok((secciones, ramos_disponibles))
}

// get_ramo_critico fue movido a `crate::excel::get_ramo_critico` para
// centralizar todo el acceso a archivos Excel en el módulo `excel`.
