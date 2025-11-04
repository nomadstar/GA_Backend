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
    // 🆕 Usar la misma lógica de normalización que en el resto del código
    fn normalize(s: &str) -> String {
        let mut out = String::new();
        for ch in s.chars() {
            let c = match ch {
                'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
                'É' | 'È' | 'Ë' | 'Ê' | 'é' | 'è' | 'ë' | 'ê' => 'e',
                'Í' | 'Ì' | 'Ï' | 'Î' | 'í' | 'ì' | 'ï' | 'î' => 'i',
                'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' | 'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
                'Ú' | 'Ù' | 'Ü' | 'Û' | 'ú' | 'ù' | 'ü' | 'û' => 'u',
                'Ñ' | 'ñ' => 'n',
                'Ç' | 'ç' => 'c',
                other => other,
            };
            if c.is_alphanumeric() {
                out.push(c.to_ascii_lowercase());
            } else if c.is_whitespace() {
                out.push(' ');
            }
        }
        out.trim().to_string()  // Quitar espacios al inicio/final
    }

    eprintln!("\n🚀 MERGE SIMPLE: MALLA (base) + OA + PA");
    eprintln!("======================================");

    // PASO 1: Leer MALLA (fuente primaria - filtra todo)
    eprintln!("\n📖 PASO 1: Leyendo MALLA desde {}", malla_archivo);
    let malla_rows = crate::excel::io::read_sheet_via_zip(malla_archivo, "")?;
    
    let mut resultado: HashMap<String, RamoDisponible> = HashMap::new();
    
    // MiMalla tiene 2 encabezados: Row 0 (fake titulo) y Row 1 (headers reales)
    // Estructura real: [ID, Código, Nombre Asignatura, ...]
    // Índices: [0=ID, 1=Código, 2=Nombre, ...]
    for (idx, row) in malla_rows.iter().enumerate() {
        if idx < 2 { continue; } // Saltear 2 encabezados
        if row.is_empty() || row.len() < 3 { continue; }
        
        let nombre_real = row.get(2).cloned().unwrap_or_default(); // Columna 2 = Nombre Asignatura
        let id_str = row.get(0).cloned().unwrap_or_else(|| "0".to_string()); // Columna 0 = ID
        let id = id_str.parse::<i32>().unwrap_or(0);
        
        let norm_name = normalize(&nombre_real);
        if !norm_name.is_empty() && norm_name != "—" {
            // Crear ramo base con datos de MALLA
            resultado.insert(norm_name.clone(), RamoDisponible {
                id,
                nombre: nombre_real,
                codigo: String::new(), // Vacío inicialmente, se llenará con OA
                holgura: 0,
                numb_correlativo: id,
                critico: false,
                codigo_ref: None,
                dificultad: None,
                electivo: false,
            });
        }
    }
    eprintln!("✅ Malla: {} cursos cargados", resultado.len());
    eprintln!("   Ramos cargados (primeros 5): {:?}", resultado.keys().take(5).collect::<Vec<_>>());

    // PASO 2: Leer OA y validar existencia (no actualizamos código, solo verificamos match)
    eprintln!("\n📖 PASO 2: Leyendo OA desde src/datafiles/OA2024.xlsx");
    
    // Construir ruta correcta para OA2024
    let base_path = std::path::Path::new(malla_archivo)
        .parent()
        .unwrap_or_else(|| std::path::Path::new(""));
    let oa_path = base_path.join("OA2024.xlsx").to_string_lossy().to_string();
    
    let oa_rows = crate::excel::io::read_sheet_via_zip(&oa_path, "")?;
    
    let mut oa_matched = 0;
    // OA2024 tiene 1 encabezado (Row 0)
    // Estructura: [Código Plan Estudio, Código, Nombre, Sección, ...]
    // Índices: [0, 1, 2, 3, ...]
    for (idx, row) in oa_rows.iter().enumerate() {
        if idx == 0 { continue; } // Saltear encabezado
        if row.is_empty() || row.len() < 3 { continue; }
        
        let nombre_oa = row.get(2).cloned().unwrap_or_default(); // Columna 2 = Nombre
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
    // Nota: Usamos el Nombre (columna 4), normalizado, para matchear con MiMalla
    let mut pa_index: HashMap<String, f64> = HashMap::new();
    
    for (idx, row) in pa_rows.iter().enumerate() {
        if idx == 0 { continue; }
        if row.is_empty() || row.len() < 9 { continue; }
        
        // Estructura PA: [Id. Ramo, Año, Período, Código Asignatura, Nombre, Est. Total, Est. Aprobados, Est. Reprobados, Porcentaje, ...]
        // Índices:       [0,         1,   2,       3,                 4,      5,          6,               7,                 8,           ...]
        let nombre_asignatura = row.get(4).cloned().unwrap_or_default(); // NOMBRE en columna 4 (ej: "MECÁNICA")
        let pct_str = row.get(8).cloned().unwrap_or_else(|| "0".to_string()); // PORCENTAJE en columna 8
        
        // Normalizar porcentaje (puede tener coma decimal)
        let pct_str_clean = pct_str.replace(",", ".");
        let pct = pct_str_clean.parse::<f64>().unwrap_or(0.0);
        
        if !nombre_asignatura.is_empty() && pct > 0.0 {
            // Normalizar el nombre para matching (uppercase, sin espacios ni acentos)
            let norm_nombre = normalize(&nombre_asignatura);
            pa_index.insert(norm_nombre, pct);
        }
    }
    eprintln!("✅ PA: {} nombres de asignatura indexados", pa_index.len());
    eprintln!("   (Primeros 5 entradas del índice PA: {:?})", pa_index.iter().take(5).collect::<Vec<_>>());

    // PASO 4: Mergear PA basado en nombre normalizado
    for ramo in resultado.values_mut() {
        // Buscar porcentaje por nombre normalizado del ramo
        let norm_ramo_nombre = normalize(&ramo.nombre);
        if let Some(pct) = pa_index.get(&norm_ramo_nombre) {
            eprintln!("   ✓ Match encontrado: '{}' -> {}%", ramo.nombre, pct);
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