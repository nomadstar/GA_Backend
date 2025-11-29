/// Módulo de extracción optimizado que usa MapeoMaestro
/// Reemplaza `extract.rs` con versión de O(n²) → O(n)

use std::collections::HashMap;
use std::error::Error;
use crate::models::{Seccion, RamoDisponible};
use crate::excel;

/// Versión optimizada de extract_data que evita búsquedas anidadas
/// 
/// Cambios clave:
/// - Usa leer_malla_con_porcentajes_optimizado en lugar de la versión lenta
/// - Una sola pasada sobre secciones en lugar de búsquedas nested
/// - Retorna exactamente lo mismo que extract_data para compatibilidad
pub fn extract_data_optimizado(
    _ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
    _sheet: Option<&str>,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    eprintln!("🚀 extract_data_optimizado: Iniciando extracción...");

    // Paso 1: Leer Malla enriquecida con porcentajes (VERSIÓN OPTIMIZADA)
    eprintln!("  📖 Paso 1: Leyendo malla con porcentajes (O(n) optimizado)...");
    
    // Usar get_datafiles_dir() para obtener la ruta correcta en runtime
    let data_dir = excel::get_datafiles_dir();
    let malla_path = data_dir.join(nombre_excel_malla).to_string_lossy().to_string();
    let porcent_str = data_dir.join("PA2025-1.xlsx").to_string_lossy().to_string();
    
    eprintln!("  📁 Rutas resueltas:");
    eprintln!("     - Malla: {}", malla_path);
    eprintln!("     - Porcentajes: {}", porcent_str);
    
    let ramos_disponibles = match excel::leer_malla_con_porcentajes_optimizado(
        &malla_path,
        &porcent_str,
    ) {
        Ok(ramos_map) => {
            eprintln!(
                "  ✅ Malla2020 enriquecida (versión optimizada): {} ramos cargados",
                ramos_map.len()
            );
            ramos_map
        }
        Err(e) => {
            eprintln!("  ⚠️  Error en leer_malla_con_porcentajes_optimizado: {}", e);
            eprintln!("  🔄 Intentando con fallback (versión antigua)...");
            match excel::leer_malla_con_porcentajes(&malla_path, &porcent_str) {
                Ok(ramos_map) => {
                    eprintln!("  ✅ Fallback exitoso: {} ramos cargados", ramos_map.len());
                    ramos_map
                }
                Err(e2) => {
                    return Err(
                        format!("Error en ambas versiones: optimizado ({}) y fallback ({})", e, e2)
                            .into(),
                    );
                }
            }
        }
    };

    // Paso 2: Leer oferta académica -> obtener secciones (UNA SOLA PASADA)
    eprintln!("  📖 Paso 2: Leyendo oferta académica (O(n) una pasada)...");
    let secciones: Vec<Seccion> = match excel::leer_oferta_academica_excel("OA2024.xlsx") {
        Ok(s) => {
            eprintln!("  ✅ Oferta académica cargada: {} secciones totales", s.len());
            s
        }
        Err(e) => {
            eprintln!(
                "  ⚠️  Error al leer oferta: {}. Usando lista vacía.",
                e
            );
            Vec::new()
        }
    };

    // Paso 3: Filtrar secciones por Malla (una sola pasada O(n))
    eprintln!("  📖 Paso 3: Filtrando secciones por Malla2020...");
    let total_secciones = secciones.len();
    let secciones_filtradas: Vec<Seccion> = secciones
        .into_iter()
        .filter(|sec| {
            // 🆕 Usar excel::normalize_name() en lugar de otra función
            let nombre_norm = crate::excel::normalize_name(&sec.nombre);
            // Aceptar si existe en ramos_disponibles (de Malla) O si es electivo
            ramos_disponibles.contains_key(&nombre_norm) || nombre_norm.contains("electivo")
        })
        .collect();

    eprintln!(
        "  ✅ Secciones filtradas: {} → {} (quedaron). Cobertura: {:.1}%",
        total_secciones,
        secciones_filtradas.len(),
        (secciones_filtradas.len() as f64 / total_secciones as f64) * 100.0
    );

    eprintln!("✅ extract_data_optimizado completado");
    Ok((secciones_filtradas, ramos_disponibles))
}

 