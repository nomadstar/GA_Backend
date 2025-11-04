//! Módulo `excel` dividido en submódulos para mantener el código organizado.
//!
//! Submódulos:
//! - `io`: helpers y utilidades para lectura/parseo de Excel
//! - `malla`: lectura de mallas curriculares
//! - `porcentajes`: lectura de porcentajes/aprobados
//! - `oferta`: lectura de oferta académica
//! - `asignatura`: búsqueda de "Asignatura" por "Nombre Asignado"
//! - `mapeo`: mapeo universal entre los 3 sistemas de códigos (Malla, OA2024, PA2025-1)

/// Helpers de IO y utilidades para parsing de Excel
mod io;

/// Lectura de malla curricular: `leer_malla_excel`
mod malla;

/// Versión optimizada de malla usando MapeoMaestro
/// Reemplaza búsquedas O(n²) con O(1) lookups
pub mod malla_optimizado;

/// Mapeo universal entre sistemas de códigos
pub mod mapeo;

/// Constructor del Mapeo Maestro (une 3 fuentes Excel)
pub mod mapeo_builder;

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
pub use malla::leer_malla_excel_with_sheet;
pub use malla::leer_prerequisitos;
pub use malla::leer_malla_con_porcentajes;
pub use malla_optimizado::leer_malla_con_porcentajes_optimizado;
pub use porcentajes::leer_porcentajes_aprobados;
pub use porcentajes::leer_porcentajes_aprobados_con_nombres;
pub use porcentajes::enrich_porcent_names_from_malla;
pub use oferta::leer_oferta_academica_excel;
pub use asignatura::asignatura_from_nombre;
pub use mapeo_builder::construir_mapeo_maestro;
pub use mapeo::{MapeoMaestro, MapeoAsignatura};
// Normalizadores expuestos para que otros módulos (algorithm, ruta) los puedan usar
// Re-exportar los helpers de normalización desde el submódulo `io` para que sean
// accesibles fuera del módulo `excel` sin exponer el módulo `io` completo.
pub use io::normalize_name;

use std::path::{Path, PathBuf};
use std::fs;
use std::error::Error;

/// Directorio protegido con los excels (relativo al repo)
/// Intenta primero la ruta desde quickshift, luego desde la raíz del proyecto
pub(crate) const DATAFILES_DIR: &str = "src/datafiles";

/// Función para resolver el directorio de datafiles correctamente
pub(crate) fn get_datafiles_dir() -> PathBuf {
    use std::path::Path;
    
    // Opción 1: Usar variable de entorno si existe
    if let Ok(path) = std::env::var("GA_DATAFILES_DIR") {
        let p = PathBuf::from(path);
        if p.exists() {
            eprintln!("✅ Usando GA_DATAFILES_DIR: {:?}", p);
            return p;
        }
    }

    // Opción 2: Buscar desde el directorio de trabajo actual (CWD)
    let cwd = match std::env::current_dir() {
        Ok(c) => c,
        Err(_) => PathBuf::from("."),
    };
    
    eprintln!("🔍 Buscando datafiles desde CWD: {:?}", cwd);
    
    let candidates_from_cwd = vec![
        cwd.join("quickshift/src/datafiles"),
        cwd.join("src/datafiles"),
        cwd.join("datafiles"),
    ];

    for candidate in candidates_from_cwd {
        if candidate.exists() {
            eprintln!("✅ Datafiles encontrados en (CWD): {:?}", candidate);
            return candidate;
        }
    }

    // Opción 3: Buscar relativo al ejecutable (para casos donde se ejecuta con ruta absoluta)
    if let Ok(exe_path) = std::env::current_exe() {
        eprintln!("🔍 Buscando relativo al ejecutable: {:?}", exe_path);
        if let Some(exe_dir) = exe_path.parent() {
            let candidates_from_exe = vec![
                exe_dir.join("../../../quickshift/src/datafiles"),
                exe_dir.join("../../quickshift/src/datafiles"),
                exe_dir.join("../quickshift/src/datafiles"),
                exe_dir.join("quickshift/src/datafiles"),
            ];
            
            for candidate in candidates_from_exe {
                if let Ok(canonical) = candidate.canonicalize() {
                    if canonical.exists() {
                        eprintln!("✅ Datafiles encontrados en (exe): {:?}", canonical);
                        return canonical;
                    }
                }
            }
        }
    }

    // Fallback: devolver ruta absoluta buscando en el sistema de archivos
    let home = match std::env::var("HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => PathBuf::from("/home/ignatus"),
    };
    
    let hardcoded = home.join("GitHub/GA_Backend/quickshift/src/datafiles");
    if hardcoded.exists() {
        eprintln!("✅ Datafiles encontrados (hardcoded): {:?}", hardcoded);
        return hardcoded;
    }
    
    eprintln!("⚠️ No se encontró directorio datafiles en ninguna ubicación");
    eprintln!("   Último intento: {:?}", hardcoded);
    hardcoded
}

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
        let name_raw = match p.file_name().and_then(|s| s.to_str()) { Some(s) => s.to_string(), None => continue };
        // ignore hidden or temporary files (editor temp like .~OA2024.xlsx, backup files ending with ~, etc.)
        if name_raw.starts_with('.') || name_raw.starts_with('~') || name_raw.ends_with('~') { continue; }
        let name = name_raw.to_lowercase();

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
    let data_dir = get_datafiles_dir();

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
                return Err(format!("malla '{}' no encontrada en cwd ni en {:?}", malla_name, data_dir).into());
            }
        }
    };

    // 2) Oferta académica: elegir el archivo más reciente que parezca OA
    let oferta_keywords = ["oferta", "oa", "oferta académica", "oferta_academica"];
    let oferta_path = latest_file_matching(&data_dir, &oferta_keywords)
        .ok_or(format!("no se encontró archivo de Oferta Académica en {}", DATAFILES_DIR))?;

    // 3) Porcentajes: elegir el archivo más reciente que parezca porcentajes de aprobación
    let porcent_keywords = ["porcentaje", "porcentajes", "porcentajeaprob", "porcentaje_aprobados"];
    let porcent_path = if let Some(p) = latest_file_matching(&data_dir, &porcent_keywords) {
        p
    } else {
        // Fallback: aceptar archivos con nombre tipo 'PA2025-1.xlsx' o que comiencen con 'pa' seguido de dígitos
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(read) = fs::read_dir(&data_dir) {
            for entry in read.flatten() {
                let p = entry.path();
                if !p.is_file() { continue; }
                let name = match p.file_name().and_then(|s| s.to_str()) { Some(s) => s.to_lowercase(), None => continue };
                // name like 'pa2025-1.xlsx' or starting with 'pa' and then a digit
                let is_pa_like = name.starts_with("pa") && name.chars().nth(2).map(|c| c.is_ascii_digit()).unwrap_or(false);
                if is_pa_like {
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
        }
        match best {
            Some((_, p)) => p,
            None => return Err(format!("no se encontró archivo de Porcentajes en {}", DATAFILES_DIR).into()),
        }
    };

    Ok((malla_path, oferta_path, porcent_path))
}

/// Lista los ficheros disponibles en `DATAFILES_DIR` categorizados como:
/// (mallas, ofertas, porcentajes). Devuelve los nombres de archivo (no paths absolutos).
pub fn list_available_datafiles() -> Result<(Vec<String>, Vec<String>, Vec<String>), Box<dyn Error>> {
    let data_dir = get_datafiles_dir();
    let mut mallas: Vec<String> = Vec::new();
    let mut ofertas: Vec<String> = Vec::new();
    let mut porcentajes: Vec<String> = Vec::new();

    let read = fs::read_dir(&data_dir)?;
    for entry in read.flatten() {
        let p = entry.path();
        if !p.is_file() { continue; }
        if let Some(name_raw) = p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()) {
            // ignore hidden or temporary files
            if name_raw.starts_with('.') || name_raw.starts_with('~') || name_raw.ends_with('~') { continue; }
            let name_low = name_raw.to_lowercase();
            if name_low.contains("malla") || name_low.contains("malla_curricular") {
                mallas.push(name_raw.clone());
            } else if name_low.contains("oferta") || name_low.contains("oa") {
                ofertas.push(name_raw.clone());
            } else if name_low.contains("porcent") || name_low.contains("aprob") || name_low.contains("porcentaje") {
                porcentajes.push(name_raw.clone());
            } else {
                // Accept PA-style filenames like 'PA2025-1.xlsx' (starts with 'pa' + digit)
                let is_pa_like = name_low.starts_with("pa") && name_low.chars().nth(2).map(|c| c.is_ascii_digit()).unwrap_or(false);
                if is_pa_like {
                    porcentajes.push(name_raw.clone());
                }
            }
        }
    }

    Ok((mallas, ofertas, porcentajes))
}

/// Lista las hojas (sheet names) internas de un workbook de malla.
/// Devuelve los nombres de las hojas en el orden que reporta la librería.
pub fn listar_hojas_malla<P: AsRef<Path>>(path: P) -> Result<Vec<String>, Box<dyn Error>> {
    // Usar calamine para abrir el workbook de forma genérica (xlsx/xls/xlsb)
    use calamine::{open_workbook_auto, Reader};
    let workbook = open_workbook_auto(path)?;
    let names = workbook.sheet_names().to_owned();
    Ok(names)
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
            let data_dir = get_datafiles_dir();
            if let Ok(entries) = std::fs::read_dir(&data_dir) {
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
            None => {
                let data_dir = get_datafiles_dir();
                let files: Vec<String> = std::fs::read_dir(&data_dir)
                    .map(|r| r.filter_map(|e| e.ok().and_then(|ent| ent.file_name().into_string().ok())).collect::<Vec<_>>())
                    .unwrap_or_default();
                panic!("No se encontró ninguna malla para el año {} en {:?}. Archivos disponibles: {:?}", year, data_dir, files)
            }
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

/// ============================================================================
/// MATCHING INTELIGENTE ENTRE TABLAS
/// ============================================================================
/// 
/// Intenta emparejar un nombre de ramo (de la malla) con nombres de la oferta
/// académica usando normalización de acentos y espacios.
///
/// Ejemplo:
/// - Nombre malla: "Mecánica"
/// - Nombre oferta: "MECÁNICA"
/// - normalize_name("Mecánica") == normalize_name("MECÁNICA") → MATCH
pub fn find_best_name_match(
    malla_name: &str,
    oferta_names: &[String],
) -> Option<String> {
    let malla_norm = normalize_name(malla_name);
    
    for oferta_name in oferta_names {
        let oferta_norm = normalize_name(oferta_name);
        if malla_norm == oferta_norm {
            return Some(oferta_name.clone());
        }
    }
    
    None
}

/// Enriquece el mapa de `ramos_disponibles` con información de oferta y porcentajes
/// usando matching por nombre normalizado.
///
/// Flujo:
/// 1. Para cada ramo en `ramos_disponibles`, normaliza su nombre
/// 2. Busca coincidencias en `oferta_secciones` por nombre normalizado
/// 3. Busca coincidencias en `porcentajes_por_nombre` por nombre normalizado
/// 4. Actualiza `dificultad` si encuentra datos de porcentajes
pub fn enrich_ramos_with_oferta_and_porcent(
    ramos_disponibles: &mut HashMap<String, RamoDisponible>,
    oferta_secciones: &[crate::models::Seccion],
    porcentajes_por_nombre: &HashMap<String, (String, f64, f64)>,
) {
    // Construir índice de oferta por nombre normalizado
    let mut oferta_por_nombre_norm: HashMap<String, Vec<&crate::models::Seccion>> = HashMap::new();
    for seccion in oferta_secciones.iter() {
        let nombre_norm = normalize_name(&seccion.nombre);
        oferta_por_nombre_norm.entry(nombre_norm).or_default().push(seccion);
    }

    // Enriquecer cada ramo
    for ramo in ramos_disponibles.values_mut() {
        let ramo_nombre_norm = normalize_name(&ramo.nombre);

        // Buscar en porcentajes por nombre normalizado
        if let Some((_codigo_origen, porc, _total)) = porcentajes_por_nombre.get(&ramo_nombre_norm) {
            ramo.dificultad = Some(*porc);
            eprintln!("DEBUG: Ramo '{}' → porcentaje encontrado: {}", ramo.nombre, porc);
        } else {
            eprintln!("DEBUG: Ramo '{}' → NO encontrado en porcentajes (norm: '{}')", ramo.nombre, ramo_nombre_norm);
        }

        // Nota: Las secciones de oferta no se usan aquí directamente para enriquecer,
        // pero se registra si hay coincidencia en oferta
        if oferta_por_nombre_norm.contains_key(&ramo_nombre_norm) {
            eprintln!("DEBUG: Ramo '{}' encontrado en oferta académica", ramo.nombre);
        }
    }
}


/// Crea un índice de nombres normalizados → nombre original para búsqueda rápida.
/// Útil para matchear Malla ↔ Oferta ↔ Porcentajes por nombre.
pub fn build_normalized_index(names: &[String]) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for name in names {
        let norm = normalize_name(name);
        index.insert(norm, name.clone());
    }
    index
}

/// Enriquece un `RamoDisponible` con información de oferta académica.
/// Intenta encontrar la mejor coincidencia por nombre normalizado.
/// 
/// Ejemplo de uso:
/// ```ignore
/// let mut ramo = RamoDisponible { nombre: "Mecánica", ... };
/// enrich_ramo_with_oferta(&mut ramo, &secciones);
/// // Ahora ramo tiene referencias a las secciones que ofrecen "Mecánica"
/// ```
pub fn enrich_ramo_with_congruencias(
    ramos: &mut HashMap<String, RamoDisponible>,
    oferta_names: &[String],
    porcentajes: &HashMap<String, (f64, f64)>,
) {
    // Crear índice de nombres normalizados en oferta
    let oferta_index = build_normalized_index(oferta_names);
    
    for (codigo, ramo) in ramos.iter_mut() {
        let ramo_norm = normalize_name(&ramo.nombre);
        
        // Buscar en oferta por nombre normalizado
        if let Some(_oferta_name) = oferta_index.get(&ramo_norm) {
            // Si encontramos la oferta, intentamos buscar porcentajes
            // usando el código del ramo o el nombre normalizado
            if let Some(&(porc, total)) = porcentajes.get(codigo) {
                ramo.dificultad = Some(porc);
            } else if let Some(&(porc, total)) = porcentajes.get(&ramo_norm) {
                ramo.dificultad = Some(porc);
            }
        }
    }
}
