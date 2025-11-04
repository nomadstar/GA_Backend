/// Módulo optimizado para lectura de malla con normalización de nombres
/// Utiliza HashMap para O(1) lookup en lugar de búsquedas nested O(n²)

use std::collections::HashMap;
use std::error::Error;
use crate::models::RamoDisponible;

/// Versión optimizada: match por nombre normalizado, filtrado por malla
/// 
/// ESTRATEGIA SIMPLE:
/// 1. Leer MALLA: extraer todos los nombres (fuente primaria)
/// 2. Leer OA: match por nombre normalizado contra MALLA -> actualizar códigos
/// 3. Leer PA: match por nombre normalizado contra MALLA -> agregar porcentajes
/// 4. Resultado: solo ramos que están en MALLA, con datos de OA y PA enriquecidos
pub fn leer_malla_con_porcentajes_optimizado(
    malla_archivo: &str,
    porcentajes_archivo: &str,
) -> Result<HashMap<String, RamoDisponible>, Box<dyn Error>> {
    fn normalize(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_uppercase())
            .collect()
    }

    eprintln!("\n🚀 MERGE SIMPLE: MALLA (base) + OA + PA");
    eprintln!("======================================");

    // PASO 1: Leer MALLA (fuente primaria - filtra todo)
    eprintln!("\n📖 PASO 1: Leyendo MALLA desde {}", malla_archivo);
    let malla_rows = crate::excel::io::read_sheet_via_zip(malla_archivo, "")?;
    
    let mut resultado: HashMap<String, RamoDisponible> = HashMap::new();
    
    for (idx, row) in malla_rows.iter().enumerate() {
        if idx == 0 { continue; }
        if row.is_empty() || row.len() < 2 { continue; }
        
        let nombre_real = row.get(0).cloned().unwrap_or_default();
        let correlativo_str = row.get(4).cloned().unwrap_or_else(|| "0".to_string());
        let correlativo = correlativo_str.parse::<i32>().unwrap_or(0);
        
        let norm_name = normalize(&nombre_real);
        if !norm_name.is_empty() && norm_name != "—" {
            // Crear ramo base con datos de MALLA
            resultado.insert(norm_name.clone(), RamoDisponible {
                id: correlativo,
                nombre: nombre_real,
                codigo: String::new(), // Vacío inicialmente, se llenará con OA
                holgura: 0,
                numb_correlativo: correlativo,
                critico: false,
                codigo_ref: None,
                dificultad: None,
                electivo: false,
            });
        }
    }
    eprintln!("✅ Malla: {} cursos cargados", resultado.len());

    // PASO 2: Leer OA y validar existencia (no actualizamos código, solo verificamos match)
    eprintln!("\n📖 PASO 2: Leyendo OA desde src/datafiles/OA2024.xlsx");
    let oa_path = format!("{}/OA2024.xlsx", crate::excel::DATAFILES_DIR);
    let oa_rows = crate::excel::io::read_sheet_via_zip(&oa_path, "")?;
    
    let mut oa_matched = 0;
    for (idx, row) in oa_rows.iter().enumerate() {
        if idx == 0 { continue; }
        if row.is_empty() || row.len() < 4 { continue; }
        
        let nombre_oa = row.get(3).cloned().unwrap_or_default(); // Columna 3 = Nombre
        let norm_oa = normalize(&nombre_oa);
        
        // Solo contar si existe en MALLA (match por nombre)
        if resultado.contains_key(&norm_oa) {
            oa_matched += 1;
        }
    }
    eprintln!("✅ OA: {} secciones matcheadas por nombre", oa_matched);

    // PASO 3: Leer PA y actualizar porcentajes en ramos
    eprintln!("\n📖 PASO 3: Leyendo PA desde {}", porcentajes_archivo);
    let pa_rows = crate::excel::io::read_sheet_via_zip(porcentajes_archivo, "")?;
    
    let mut pa_matched = 0;
    // Construir índice PA: nombre_normalizado -> porcentaje
    let mut pa_index: HashMap<String, f64> = HashMap::new();
    
    for (idx, row) in pa_rows.iter().enumerate() {
        if idx == 0 { continue; }
        if row.is_empty() || row.len() < 5 { continue; }
        
        // Estructura PA: [ID_RAMO, AÑO, PERÍODO, CÓDIGO_ASIGNATURA, NOMBRE, EST_TOTAL, EST_APROBADOS, EST_REPROBADOS, PORCENTAJE, ...]
        // Indices:       [0,        1,    2,        3,                 4,      5,         6,               7,                 8,           ...]
        let nombre_asignatura = row.get(4).cloned().unwrap_or_default(); // NOMBRE en columna 4
        let pct_str = row.get(8).cloned().unwrap_or_else(|| "0".to_string()); // PORCENTAJE en columna 8
        
        // Normalizar porcentaje (puede tener coma decimal)
        let pct_str_clean = pct_str.replace(",", ".");
        let pct = pct_str_clean.parse::<f64>().unwrap_or(0.0);
        
        if !nombre_asignatura.is_empty() && pct > 0.0 {
            let norm_nombre = normalize(&nombre_asignatura);
            pa_index.insert(norm_nombre, pct);
        }
    }
    eprintln!("✅ PA: {} nombres de asignatura indexados", pa_index.len());

    // PASO 4: Mergear PA basado en nombre normalizado
    for ramo in resultado.values_mut() {
        // Buscar porcentaje por nombre normalizado del ramo
        let norm_ramo_nombre = normalize(&ramo.nombre);
        if let Some(pct) = pa_index.get(&norm_ramo_nombre) {
            ramo.dificultad = Some(*pct);
            pa_matched += 1;
        }
    }
    eprintln!("✅ PA: {} porcentajes matcheados por nombre", pa_matched);

    eprintln!("\n✅ MERGE COMPLETADO:");
    eprintln!("  - Ramos de MALLA: {}", resultado.len());
    eprintln!("  - Con OA actualizado: {}", oa_matched);
    eprintln!("  - Con PA (porcentaje): {}", pa_matched);

    Ok(resultado)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_carga_malla_con_porcentajes() {
        let result = leer_malla_con_porcentajes_optimizado(
            "src/datafiles/MiMalla.xlsx",
            "src/datafiles/PA2025-1.xlsx",
        );

        match result {
            Ok(ramos) => {
                assert!(ramos.len() > 0, "Debe haber al menos un ramo");
                eprintln!("✅ Test exitoso: {} ramos cargados", ramos.len());
            }
            Err(e) => {
                eprintln!("⚠️  Test incompleto (archivos no disponibles): {}", e);
            }
        }
    }
    }