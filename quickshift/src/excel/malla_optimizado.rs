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
    eprintln!("🔍 [OPTIMIZED MALLA] Starting - malla_archivo={}", malla_archivo);
    
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
    
    // Detectar qué hoja leer: si es MiMalla.xlsx usa "Malla2020", si es Malla2020.xlsx usa "" (hoja activa)
    let sheet_name = if malla_archivo.contains("MiMalla") || malla_archivo.contains("mimalla") {
        "Malla2020"
    } else {
        "" // Usar la hoja activa (Sheet1)
    };
    eprintln!("   Usando hoja: '{}'", if sheet_name.is_empty() { "Sheet1 (activa)" } else { sheet_name });
    
    let malla_rows = crate::excel::io::read_sheet_via_zip(malla_archivo, sheet_name)?;
    
    let mut resultado: HashMap<String, RamoDisponible> = HashMap::new();

    // Detectar fila de encabezado y columnas (nombre / id / semestre / requisitos) de forma robusta
    let mut header_row_idx: Option<usize> = None;
    let mut name_col_idx: usize = 2; // fallback antiguo
    let mut id_col_idx: usize = 0; // fallback antiguo
    let mut semestre_col_idx: Option<usize> = None; // Nueva columna
    let mut requisitos_col_idx: Option<usize> = None; // Columna para leer requisitos previos
    
    eprintln!("DEBUG: malla_rows.len()={}", malla_rows.len());
    if !malla_rows.is_empty() {
        eprintln!("DEBUG: First row (header): {:?}", malla_rows.get(0));
    }
    
    for (i, row) in malla_rows.iter().enumerate().take(4) {
        // buscar palabras clave en las celdas
        for (j, cell) in row.iter().enumerate() {
            let lower = cell.to_lowercase();
            eprintln!("DEBUG: Row {}, Col {}: '{}' -> '{}'", i, j, cell, lower);
            
            // Detectar columna de NOMBRE pero evitar confundir con columnas como
            // "Abre la/s asignatura/s" que contienen listas de referencias.
            if lower.contains("nombre") || (lower.contains("asignatura") && !lower.contains("abre")) || lower.contains("curso") {
                // sólo asignar si no fue detectada antes (preferir la primera aparición)
                if header_row_idx.is_none() || name_col_idx == 2 /* fallback */ {
                    header_row_idx = Some(i);
                    name_col_idx = j;
                }
            }
            if lower.contains("id") || lower.contains("ident") || lower.contains("codigo") || lower.contains("código") {
                header_row_idx = Some(i);
                // si aún no tenemos id_col, tomar este
                id_col_idx = j;
            }
            if lower.contains("semestre") {
                header_row_idx = Some(i);
                semestre_col_idx = Some(j);
                eprintln!("DEBUG: Found 'semestre' at row {} col {}", i, j);
            }
            if lower.contains("requisito") {
                header_row_idx = Some(i);
                requisitos_col_idx = Some(j);
                eprintln!("DEBUG: Found 'requisitos' at row {} col {}", i, j);
            }
        }
    }

    let start_idx = match header_row_idx {
        Some(h) => h + 1,
        None => 2, // comportamiento legacy
    };

    eprintln!("DEBUG: Malla header detected at {:?}, using name_col={} id_col={} semestre_col={:?} requisitos_col={:?}", header_row_idx, name_col_idx, id_col_idx, semestre_col_idx, requisitos_col_idx);

    for (idx, row) in malla_rows.iter().enumerate() {
        if idx < start_idx { continue; }
        if row.is_empty() || row.len() <= name_col_idx { continue; }

        let nombre_real = row.get(name_col_idx).cloned().unwrap_or_default();
        let id_str = row.get(id_col_idx).cloned().unwrap_or_else(|| "0".to_string());
        let id = id_str.parse::<i32>().unwrap_or(0);
        
        // Leer semestre si está disponible
        let semestre_opt = semestre_col_idx.and_then(|col| {
            row.get(col).and_then(|sem_str| {
                sem_str.trim().parse::<i32>().ok()
            })
        });
        
        // Leer requisitos si está disponible (IDs de ramos prerequisitos)
        // Formato: puede ser "1", "1.2", "1,2", etc.
        let requisitos_ids = requisitos_col_idx.and_then(|col| {
            row.get(col).and_then(|req_str| {
                let trimmed = req_str.trim();
                // Si es "—" o vacío, no hay requisito
                if trimmed.is_empty() || trimmed == "—" {
                    return Some(vec![]);
                }
                
                // Parsear múltiples IDs separados por . o ,
                let ids: Vec<i32> = trimmed
                    .split(|c| c == '.' || c == ',')
                    .filter_map(|s| s.trim().parse::<i32>().ok())
                    .collect();
                
                if ids.is_empty() {
                    None
                } else {
                    Some(ids)
                }
            })
        }).unwrap_or_default();

        let norm_name = normalize(&nombre_real);
        if !norm_name.is_empty() && norm_name != "—" {
            resultado.insert(norm_name.clone(), RamoDisponible {
                id,
                nombre: nombre_real,
                codigo: String::new(),
                holgura: 0,
                numb_correlativo: id,
                critico: false,
                requisitos_ids,  // Ahora usa múltiples IDs
                dificultad: None,
                electivo: false,
                semestre: semestre_opt,
            });
        }
    }
    eprintln!("✅ Malla: {} cursos cargados", resultado.len());
    eprintln!("   Ramos cargados (primeros 5): {:?}", resultado.keys().take(5).collect::<Vec<_>>());
    
    // Log de requisitos leídos
    eprintln!("   Requisitos detectados:");
    for (_name, ramo) in resultado.iter().take(15) {
        if !ramo.requisitos_ids.is_empty() {
            eprintln!("     - {} (id={}) -> requisitos ids={:?}", ramo.nombre, ramo.id, ramo.requisitos_ids);
        }
    }

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
        
        let codigo_oa = row.get(1).cloned().unwrap_or_default(); // Columna 1 = Código
        let nombre_oa = row.get(2).cloned().unwrap_or_default(); // Columna 2 = Nombre
        let norm_oa = normalize(&nombre_oa);
        
        // Solo contar si existe en MALLA (match por nombre)
        // Y actualizar el código si no estaba ya seteado
        if let Some(ramo) = resultado.get_mut(&norm_oa) {
            if ramo.codigo.is_empty() && !codigo_oa.is_empty() {
                ramo.codigo = codigo_oa;
                oa_matched += 1;
            }
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

