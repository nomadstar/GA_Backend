//! Módulo `excel` dividido en submódulos para mantener el código organizado.
//!
//! Submódulos:
//! - `io`: helpers y utilidades para lectura/parseo de Excel
//! - `malla`: lectura de mallas curriculares
//! - `porcentajes`: lectura de porcentajes/aprobados
//! - `oferta`: lectura de oferta académica
//! - `asignatura`: búsqueda de "Asignatura" por "Nombre Asignado"

/// Helpers de IO y utilidades para parsing de Excel
mod io;

/// Lectura de malla curricular: `leer_malla_excel`
mod malla;

/// Lectura de porcentajes/aprobados: `leer_porcentajes_aprobados`
mod porcentajes;

/// Lectura de oferta académica: `leer_oferta_academica_excel`
mod oferta;

/// Búsqueda de "Asignatura" a partir de "Nombre Asignado": `asignatura_from_nombre`
mod asignatura;

// Re-exports: helpers de IO son internos al crate; exponemos sólo las funciones de alto nivel
// helpers internos — no exportarlos públicamente
// funciones de alto nivel que sí usa `algorithm`
pub use malla::leer_malla_excel;
pub use malla::leer_prerequisitos;
pub use porcentajes::leer_porcentajes_aprobados;
pub use oferta::leer_oferta_academica_excel;
pub use asignatura::asignatura_from_nombre;

use std::path::{Path, PathBuf};
use std::fs;
use std::error::Error;

/// Directorio protegido con los excels (relativo al repo)
pub(crate) const DATAFILES_DIR: &str = "src/datafiles";

use crate::models::RamoDisponible;
use std::collections::HashMap;

/// Intento práctico de obtener el mapa inicial de ramos a partir de una malla
/// por defecto. Mantiene la misma firma usada anteriormente en `algorithm`.
/// Devuelve (mapa, nombre_malla, leido_flag).
pub fn get_ramo_critico(nombre: &str) -> (HashMap<String, RamoDisponible>, String, bool) {
    match leer_malla_excel(nombre) {
        Ok(map) => (map, nombre.to_string(), true),
        Err(_) => (HashMap::new(), nombre.to_string(), false),
    }
}

fn latest_file_matching(dir: &Path, keywords: &[&str]) -> Option<PathBuf> {
    let read = match fs::read_dir(dir) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;

    for entry in read.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        let name = match p.file_name().and_then(|s| s.to_str()) { Some(s) => s.to_lowercase(), None => continue };

        if keywords.iter().any(|kw| name.contains(&kw.to_lowercase())) {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    match &best {
                        Some((best_time, _)) if *best_time >= modified => (),
                        _ => best = Some((modified, p.clone())),
                    }
                }
            }
        }
    }

    best.map(|(_, p)| p)
}

/// Resuelve las rutas de datos: (malla_path, oferta_path, porcentajes_path)
/// - malla_name puede ser nombre de archivo o path absoluto; si no existe, buscar en DATAFILES_DIR.
/// - Devuelve error si no encuentra alguno de los tres archivos.
pub fn resolve_datafile_paths(malla_name: &str) -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn Error>> {
    let data_dir = Path::new(DATAFILES_DIR);

    // 1) Malla: preferir path directo, si no buscar en data_dir
    let malla_path = {
        let maybe = Path::new(malla_name);
        if maybe.exists() && maybe.is_file() {
            maybe.to_path_buf()
        } else {
            let candidate = data_dir.join(malla_name);
            if candidate.exists() && candidate.is_file() {
                candidate
            } else {
                return Err(format!("malla '{}' no encontrada en cwd ni en {}", malla_name, DATAFILES_DIR).into());
            }
        }
    };

    // 2) Oferta académica: elegir el archivo más reciente que parezca OA
    let oferta_keywords = ["oferta", "oa", "oferta académica", "oferta_academica"];
    let oferta_path = latest_file_matching(data_dir, &oferta_keywords)
        .ok_or(format!("no se encontró archivo de Oferta Académica en {}", DATAFILES_DIR))?;

    // 3) Porcentajes: elegir el archivo más reciente que parezca porcentajes de aprobación
    let porcent_keywords = ["porcentaje", "porcentajes", "porcentajeaprob", "porcentaje_aprobados"];
    let porcent_path = latest_file_matching(data_dir, &porcent_keywords)
        .ok_or(format!("no se encontró archivo de Porcentajes en {}", DATAFILES_DIR))?;

    Ok((malla_path, oferta_path, porcent_path))
}


#[cfg(test)]
mod tests {
    use super::*;

    // Test helper: dado un año (p.ej. 2020) intenta resolver la malla y la imprime.
    #[test]
    fn test_print_malla_by_year() {
        // Cambia este valor para probar distinto año desde el test
        let year = 2020i32;
        // Intentamos varios patrones comunes (MallaCurricular{year}, MiMalla{year}, MiMalla)
        let candidate1 = format!("MallaCurricular{}.xlsx", year);
        let candidate2 = format!("MiMalla{}.xlsx", year);
        let candidate3 = "MiMalla.xlsx".to_string();

        let mut resolved_malla: Option<std::path::PathBuf> = None;
        for cand in &[candidate1.clone(), candidate2.clone(), candidate3.clone()] {
            if let Ok((m, _o, _p)) = resolve_datafile_paths(cand) {
                resolved_malla = Some(m);
                break;
            }
        }

        // Si no encontramos por patrón, buscar cualquier fichero en DATAFILES_DIR que contenga el año
        if resolved_malla.is_none() {
            let data_dir = std::path::Path::new(DATAFILES_DIR);
            if let Ok(entries) = std::fs::read_dir(data_dir) {
                for e in entries.flatten() {
                    if let Some(name) = e.file_name().to_str() {
                        if name.contains(&year.to_string()) {
                            resolved_malla = Some(e.path());
                            break;
                        }
                    }
                }
            }
        }

        let malla_path = match resolved_malla {
            Some(p) => p,
            None => panic!("No se encontró ninguna malla para el año {} en {}. Archivos disponibles: {:?}", year, DATAFILES_DIR, std::fs::read_dir(DATAFILES_DIR).map(|r| r.filter_map(|e| e.ok().and_then(|ent| ent.file_name().into_string().ok())).collect::<Vec<_>>()).unwrap_or_default()),
        };

        let malla_str = malla_path.to_str().expect("malla path no UTF-8");
        let map = leer_malla_excel(malla_str).expect("falló leer_malla_excel");

        // Aserción mínima: la malla no debe estar vacía
        assert!(!map.is_empty(), "La malla leída está vacía para {}", malla_str);

        // Imprimir las primeras entradas para inspección humana
        println!("Malla leída desde: {} -> {} ramos", malla_str, map.len());
        for (codigo, ramo) in map.iter().take(50) {
            println!("{} => {}", codigo, ramo.nombre);
        }
    }
}
