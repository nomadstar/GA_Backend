/// Constructor del Mapeo Maestro
/// Lee los 3 archivos Excel (Malla2020, OA2024, PA2025-1) y construye un mapa
/// unificado donde cada asignatura se identifica por su NOMBRE NORMALIZADO.

use crate::excel::mapeo::{MapeoMaestro, MapeoAsignatura};
use crate::excel::{malla, oferta, porcentajes, normalize_name};
use crate::excel::io::data_to_string;
use calamine::{open_workbook_auto, Data, Reader};
use std::collections::HashMap;
use std::path::Path;

/// Construir mapeo maestro desde los 3 archivos Excel
pub fn construir_mapeo_maestro(
    ruta_malla: &str,
    ruta_oa2024: &str,
    ruta_pa2025: &str,
) -> Result<MapeoMaestro, Box<dyn std::error::Error>> {
    let mut mapeo = MapeoMaestro::new();

    // PASO 1: Leer PA2025-1 (es la fuente de verdad para códigos y porcentajes)
    eprintln!("📖 PASO 1: Leyendo PA2025-1...");
    leer_pa2025_al_mapeo(ruta_pa2025, &mut mapeo)?;

    // PASO 2: Leer OA2024 (agrega información de horarios/secciones)
    eprintln!("📖 PASO 2: Leyendo OA2024...");
    leer_oa2024_al_mapeo(ruta_oa2024, &mut mapeo)?;

    // PASO 3: Leer Malla2020 (agrega información de estructura y dependencias)
    eprintln!("📖 PASO 3: Leyendo Malla2020...");
    leer_malla2020_al_mapeo(ruta_malla, &mut mapeo)?;

    eprintln!("✅ {}", mapeo.resumen());
    Ok(mapeo)
}

/// Leer PA2025-1 y agregar al mapeo
fn leer_pa2025_al_mapeo(
    archivo: &str,
    mapeo: &mut MapeoMaestro,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = if Path::new(archivo).exists() {
        archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, archivo);
        if Path::new(&candidate).exists() { candidate } else { archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(&resolved)?;
    let sheet_name = workbook.sheet_names()[0].clone();
    let range = workbook.worksheet_range(&sheet_name)?;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; } // Skip header

        // PA2025-1: Columnas = Id.Ramo | Año | Período | Código | Nombre | Est.Total | Est.Aprob | Est.Reprob | Porcentaje | Porcentaje Reprob | Electivo
        let id_ramo = data_to_string(row.get(0).unwrap_or(&Data::Empty)).trim().to_string();
        let codigo = data_to_string(row.get(3).unwrap_or(&Data::Empty)).trim().to_string();
        let nombre = data_to_string(row.get(4).unwrap_or(&Data::Empty)).trim().to_string();
        let porcentaje_str = data_to_string(row.get(8).unwrap_or(&Data::Empty)).trim().to_string();
        let es_electivo_str = data_to_string(row.get(10).unwrap_or(&Data::Empty)).trim().to_lowercase();

        if nombre.is_empty() || codigo.is_empty() { continue; }

        let nombre_norm = normalize_name(&nombre);
        let porcentaje = porcentaje_str.parse::<f64>().ok();
        let es_electivo = es_electivo_str == "true" || es_electivo_str == "1";

        let mut asignatura = MapeoAsignatura::new(nombre_norm, nombre);
        asignatura.codigo_pa2025 = Some(codigo);
        asignatura.porcentaje_aprobacion = porcentaje;
        asignatura.es_electivo = es_electivo;

        mapeo.add_asignatura(asignatura);
    }

    eprintln!("  ✓ PA2025-1: {} asignaturas cargadas", mapeo.len());
    Ok(())
}

/// Leer OA2024 y agregar/actualizar al mapeo
fn leer_oa2024_al_mapeo(
    archivo: &str,
    mapeo: &mut MapeoMaestro,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = if Path::new(archivo).exists() {
        archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, archivo);
        if Path::new(&candidate).exists() { candidate } else { archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(&resolved)?;
    let sheet_name = workbook.sheet_names()[0].clone();
    let range = workbook.worksheet_range(&sheet_name)?;

    let mut contador = 0;
    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; } // Skip header

        // OA2024: Columna 1 = Código, Columna 2 = Nombre
        let codigo = data_to_string(row.get(1).unwrap_or(&Data::Empty)).trim().to_string();
        let nombre = data_to_string(row.get(2).unwrap_or(&Data::Empty)).trim().to_string();

        if nombre.is_empty() || codigo.is_empty() { continue; }

        let nombre_norm = normalize_name(&nombre);

        // Si ya existe en el mapeo (de PA2025-1), actualizar con código de OA2024
        if let Some(asignatura_mut) = mapeo.asignaturas.get_mut(&nombre_norm) {
            asignatura_mut.codigo_oa2024 = Some(codigo);
        }

        contador += 1;
    }

    eprintln!("  ✓ OA2024: {} secciones procesadas", contador);
    Ok(())
}

/// Leer Malla2020 y agregar/actualizar al mapeo
fn leer_malla2020_al_mapeo(
    archivo: &str,
    mapeo: &mut MapeoMaestro,
) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = if Path::new(archivo).exists() {
        archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, archivo);
        if Path::new(&candidate).exists() { candidate } else { archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(&resolved)?;
    let range = workbook.worksheet_range("Malla2020")?;

    let mut contador = 0;
    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; } // Skip header

        // Malla2020: Columna 0 = Nombre, Columna 1 = ID
        let nombre = data_to_string(row.get(0).unwrap_or(&Data::Empty)).trim().to_string();
        let id_str = data_to_string(row.get(1).unwrap_or(&Data::Empty)).trim().to_string();

        if nombre.is_empty() || id_str.is_empty() { continue; }

        let id = id_str.parse::<i32>().ok();
        let nombre_norm = normalize_name(&nombre);

        // Si existe en el mapeo, actualizar con ID de Malla
        if let Some(asignatura_mut) = mapeo.asignaturas.get_mut(&nombre_norm) {
            asignatura_mut.id_malla = id;
        }

        contador += 1;
    }

    eprintln!("  ✓ Malla2020: {} asignaturas procesadas", contador);
    Ok(())
}

// Necesitamos acceso mutable a HashMap en MapeoMaestro para actualizar
// Esto requiere cambiar MapeoMaestro para tener un método `get_mut` o similar
// Para ahora, vamos a usar una estructura temporal interna

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construir_mapeo() {
        // Este test se ejecutaría si tuviéramos acceso a los archivos
        // Por ahora es solo un placeholder
    }
}
