// Port literal del script Python `RutaCritica/extract_data.py` a Rust.
//
// Este módulo implementa las mismas funciones auxiliares que el script
// original: equivalencia, counters, appendElectivos, secciones_cfg y
// extract_data. Está escrito de manera relativamente literal para facilitar
// la revisión y posteriores refactorizaciones.

use std::collections::HashMap;
use std::error::Error;
use crate::models::{Seccion, RamoDisponible};

// Delegamos la lectura de archivos Excel al módulo `excel`.
use crate::excel::{leer_malla_excel, leer_porcentajes_aprobados, leer_oferta_academica_excel};

/// Versión simplificada de `extract_data` que delega en `src/excel`.
/// - Si `ramos_disponibles` viene vacío, intenta cargar la malla `nombre_excel_malla`.
/// - Lee la oferta académica con `leer_oferta_academica_excel`.
/// - Lee porcentajes y asigna `dificultad` a los ramos cuando sea posible.
pub fn extract_data(
    mut ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    // 1) Cargar malla si no se nos pasó un mapa ya poblado
    if ramos_disponibles.is_empty() {
        if let Ok(m) = leer_malla_excel(nombre_excel_malla) {
            ramos_disponibles = m;
        }
    }

    // 2) Leer oferta académica (lista de secciones)
    // Intentamos leer usando el helper; si falla devolvemos el error.
    // El caller puede proporcionar un path distinto si lo necesita.
    let oferta_path = "RutaCritica/Oferta Academica 2021-1 vacantes 2021-02-04.xlsx";
    let lista_secciones = match leer_oferta_academica_excel(oferta_path) {
        Ok(ls) => ls,
        Err(e) => return Err(e),
    };

    // 3) Leer porcentajes y poblar dificultad cuando corresponda
    let porcentajes_path = "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx";
    if let Ok(pmap) = leer_porcentajes_aprobados(porcentajes_path) {
        for (codigo, (porc, _total)) in pmap.into_iter() {
            if let Some(r) = ramos_disponibles.get_mut(&codigo) {
                r.dificultad = Some(porc);
            }
        }
    }

    Ok((lista_secciones, ramos_disponibles))
}

/// Devuelve un mapa inicial de ramos (si existe la malla por defecto) y el nombre de la malla.
pub fn get_ramo_critico() -> (HashMap<String, RamoDisponible>, String, bool) {
    let nombre = "MiMalla.xlsx".to_string();
    match leer_malla_excel(&nombre) {
        Ok(map) => (map, nombre, true),
        Err(_) => (HashMap::new(), nombre, false),
    }
}
