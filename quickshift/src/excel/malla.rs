use std::collections::{HashMap, HashSet};
use calamine::{open_workbook_auto, Data, Reader};
use crate::models::RamoDisponible;
use crate::excel::io::data_to_string;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

// Índices configurables (se pueden cambiar en tiempo de ejecución si se desea)
pub static MALLA_NAME_COL: AtomicUsize = AtomicUsize::new(0);
pub static MALLA_ID_COL: AtomicUsize = AtomicUsize::new(1);
pub static OA_NAME_COL: AtomicUsize = AtomicUsize::new(2);
pub static OA_CODE_COL: AtomicUsize = AtomicUsize::new(0);

/// Lee un archivo de malla (espera filas: codigo, nombre, correlativo, holgura, critico, ...)
/// Leer malla desde un archivo Excel, permitiendo opcionalmente elegir la hoja
/// por nombre. Si `sheet` es None se usa la primera hoja del workbook.
pub fn leer_malla_excel_with_sheet(nombre_archivo: &str, sheet: Option<&str>) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    // Resolver ruta: si el path directo no existe, intentar buscar en el directorio protegido `DATAFILES_DIR`
    let resolved = if std::path::Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, nombre_archivo);
        if std::path::Path::new(&candidate).exists() { candidate } else { nombre_archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(resolved)?;
    let mut ramos_disponibles = HashMap::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    // Elegir hoja: prioridad -> sheet (si provisto y existe), else primera hoja
    let hoja_seleccionada = if let Some(s) = sheet {
        if sheet_names.iter().any(|n| n == s) { s.to_string() } else { sheet_names[0].clone() }
    } else {
        sheet_names[0].clone()
    };

    let range = workbook.worksheet_range(&hoja_seleccionada)?;

    // Detectar índices de columnas por encabezado (si existe)
    let mut name_idx: usize = 0;
    let mut id_idx: usize = 1;
    let mut rows: Vec<_> = range.rows().collect();
    if !rows.is_empty() {
        let header = rows[0];
        for (i, cell) in header.iter().enumerate() {
            let s = data_to_string(cell).to_lowercase();
            if s.contains("nombre") || s.contains("asignatura") || s.contains("curso") {
                name_idx = i;
            }
            if s.contains("código") || s.contains("codigo") || s.contains("id") {
                id_idx = i;
            }
        }
        eprintln!("DEBUG: header detected -> name_idx={} id_idx={}", name_idx, id_idx);
    }

    // Iterar filas usando los índices detectados; saltar header si existe
    for (row_idx, row) in rows.into_iter().enumerate() {
        if row_idx == 0 {
            // si la fila 0 parece ser header (contiene "nombre" o "id"), la saltamos
            let col0 = data_to_string(row.get(0).unwrap_or(&Data::Empty)).to_lowercase();
            if col0.contains("nombre") || col0.contains("código") || col0.contains("id") { continue; }
        }

        // Leer columnas usando índices detectados
        let raw_name = data_to_string(row.get(name_idx).unwrap_or(&Data::Empty)).trim().to_string();
        let raw_id = data_to_string(row.get(id_idx).unwrap_or(&Data::Empty)).trim().to_string();

        // Si por alguna razón el nombre está vacío pero otra columna parece textual, intentar fallback
        let nombre = if raw_name.is_empty() {
            // buscar la primera celda no-vacía que parezca texto
            row.iter()
                .map(|c| data_to_string(c))
                .find(|s| !s.is_empty() && s.chars().any(|ch| ch.is_alphabetic()))
                .unwrap_or_else(|| raw_name.clone())
        } else {
            raw_name.clone()
        };

        let id_str = if raw_id.is_empty() {
            // fallback: buscar primera celda numérica razonable
            row.iter()
                .map(|c| data_to_string(c))
                .find(|s| !s.is_empty() && s.chars().all(|ch| ch.is_numeric()))
                .unwrap_or_else(|| raw_id.clone())
        } else {
            raw_id.clone()
        };

        let id = id_str.parse::<i32>().unwrap_or(0);

        if nombre.is_empty() || id == 0 {
            // ignorar filas incompletas
            continue;
        }

        let nombre_norm = crate::excel::normalize_name(&nombre);
        ramos_disponibles.insert(nombre_norm, RamoDisponible {
            id,
            nombre,
            codigo: id_str.clone(),
            holgura: 0,
            numb_correlativo: id,
            critico: false,
            requisitos_ids: vec![],
            dificultad: None,
            electivo: false,
            semestre: None,
        });
    }

    Ok(ramos_disponibles)
}

/// Normaliza el par (col0, col1) devolviendo (codigo, nombre).
/// Si detecta que la primera columna contiene letras y la segunda contiene
/// dígitos (por ejemplo: "Nombre" | "ID"), invierte el orden para que el
/// resultado sea siempre (ID, Nombre).
pub fn normalize_codigo_nombre(col0: &str, col1: &str) -> (String, String) {
    let mut codigo = col0.to_string();
    let mut nombre = col1.to_string();
    let first_has_alpha = codigo.chars().any(|c| c.is_alphabetic());
    let second_has_digit = nombre.chars().any(|c| c.is_digit(10));
    if first_has_alpha && second_has_digit {
        std::mem::swap(&mut codigo, &mut nombre);
    }
    (codigo, nombre)
}



/// Compat wrapper existente que conserva el nombre original y usa la primera hoja
pub fn leer_malla_excel(nombre_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    leer_malla_excel_with_sheet(nombre_archivo, None)
}

/// Lee hojas adicionales de la malla para extraer prerequisitos.
/// Se espera que cada hoja adicional tenga al menos dos columnas:
/// - columna 0: codigo de la asignatura
/// - columna 1: prerequisitos (puede contener varios códigos separados por ',' o ';')
pub fn leer_prerequisitos(nombre_archivo: &str) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    // Resolver ruta: si el path directo no existe, intentar buscar en el directorio protegido `DATAFILES_DIR`
    let resolved = if Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, nombre_archivo);
        if Path::new(&candidate).exists() { candidate } else { nombre_archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(resolved)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Ok(map);
    }

    // Si el archivo es una de las Mallas históricas, leer los prerequisitos
    // desde la PRIMERA hoja (algunos archivos ponen los prerequisitos ahí).
    // Para otros archivos, mantenemos el comportamiento anterior (hojas a partir de la segunda).
    let filename_lower = Path::new(nombre_archivo)
        .file_name()
        .and_then(|os| os.to_str())
        .unwrap_or("")
        .to_lowercase();

    let special_first_sheet = filename_lower.contains("malla2020")
        || filename_lower.contains("malla2018")
        || filename_lower.contains("malla2010");

    let sheets_to_iterate: Vec<String> = if special_first_sheet {
        vec![sheet_names[0].clone()]
    } else {
        if sheet_names.len() <= 1 {
            // no hay hojas adicionales con prerequisitos
            return Ok(map);
        }
        sheet_names.iter().skip(1).cloned().collect()
    };

    // Iterar sobre las hojas seleccionadas y extraer pares (codigo -> [prereqs])
    for sheet in sheets_to_iterate.iter() {
        if let Ok(range) = workbook.worksheet_range(sheet) {
            // Detectar índices de columnas en la hoja de prereqs (si existe header)
            let mut codigo_col: usize = 0;
            let mut prereq_col: usize = 1; // fallback histórico
            let rows: Vec<_> = range.rows().collect();
            if !rows.is_empty() {
                let header = rows[0];
                for (i, cell) in header.iter().enumerate() {
                    let s = data_to_string(cell).to_lowercase();
                    if s.contains("código") || s.contains("codigo") || s.contains("id") {
                        codigo_col = i;
                    }
                    if s.contains("requisito") || s.contains("requisitos") || s.contains("requerimiento") {
                        prereq_col = i;
                    }
                }
            }

            for (row_idx, row) in range.rows().enumerate() {
                if row_idx == 0 { continue; }
                let codigo = data_to_string(row.get(codigo_col).unwrap_or(&Data::Empty));
                let raw_pr = data_to_string(row.get(prereq_col).unwrap_or(&Data::Empty));
                if codigo.is_empty() || raw_pr.is_empty() { continue; }
                // separar por comas o punto y coma
                let mut list: Vec<String> = raw_pr.split(|c| c==',' || c==';')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !list.is_empty() {
                    map.entry(codigo.clone()).or_insert_with(Vec::new).append(&mut list);
                }
            }
        }
    }

    Ok(map)
}

/// Lee Malla2020 y lo enriquece con información de PA2025-1 (porcentajes y códigos)
/// 
/// IMPORTANTE: Manejo especial de ELECTIVOS
/// Los electivos se repiten en Malla2020 (ej: "Electivo Profesional" con múltiples IDs)
/// Por eso indexamos diferente:
/// - NO-ELECTIVOS: clave = nombre_normalizado (universal)
/// - ELECTIVOS: clave = codigo de PA2025-1 (único para cada opción de electivo)
/// 
/// Flujo:
/// 1. Lee PA2025-1 para extraer mapeo: nombre_normalizado → (código, porcentaje, total, es_electivo)
/// 2. Lee Malla2020 (Nombre, ID, Créditos, Requisitos, Semestre, Electivo)
/// 3. Por cada ramo en Malla2020:
///    a. Si es NO-ELECTIVO: normaliza nombre y busca en PA2025-1
///    b. Si es ELECTIVO: busca todos los códigos en PA2025-1 con Electivo=TRUE
///       y selecciona el que tenga MEJOR porcentaje (menor tasa de reprobación)
/// 4. SEGUNDO PASE: Resuelve dependencias por ID
/// 
/// Retorna: HashMap con claves diferenciadas:
/// - NO-ELECTIVOS: nombre_normalizado
/// - ELECTIVOS: codigo de PA2025-1 (ej: "CIT2020", "CBF1001")
pub fn leer_malla_con_porcentajes(malla_archivo: &str, porcentajes_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
     use crate::excel::normalize_name;
     use crate::excel::porcentajes::leer_porcentajes_aprobados_con_nombres;
     
     // 1. Leer porcentajes y construir índice por nombre normalizado
     let (_porcent_by_code, porcent_by_name) = leer_porcentajes_aprobados_con_nombres(porcentajes_archivo)?;
     
    // 2. Intentar leer OA2024 para obtener lista de NOMBRES (NO usar códigos)
    // Usamos un HashSet de nombres normalizados.
    let mut oa_nombres: HashSet<String> = HashSet::new();
     let mut resolved_malla_path: Option<std::path::PathBuf> = None;
    if let Ok((malla_path, oferta_path, _)) = crate::excel::resolve_datafile_paths(malla_archivo) {
            // Intentar abrir con calamine y detectar columna de nombre dinámicamente
            if let Ok(mut workbook) = calamine::open_workbook_auto(oferta_path.to_str().unwrap_or("")) {
                let sheet_names = workbook.sheet_names().to_owned();
                for sheet in sheet_names.iter() {
                    if let Ok(range) = workbook.worksheet_range(sheet) {
                        // Detectar columna de nombre en header (si existe)
                        let mut oa_name_col: usize = OA_NAME_COL.load(Ordering::Relaxed);
                        let rows_vec: Vec<_> = range.rows().collect();
                        if let Some(header_row) = rows_vec.get(0) {
                            for (i, cell) in header_row.iter().enumerate() {
                                let s = data_to_string(cell).to_lowercase();
                                if s.contains("nombre") || s.contains("asignatura") || s.contains("ramo") {
                                    oa_name_col = i;
                                }
                            }
                            eprintln!("DEBUG: OA header detected in '{}' -> oa_name_col={}", sheet, oa_name_col);
                        }

                        let mut oa_debug_count = 0;
                        for (row_idx, row) in rows_vec.into_iter().enumerate() {
                            if row_idx == 0 { continue; }  // skip header
                            let nombre = data_to_string(row.get(oa_name_col).unwrap_or(&Data::Empty)).trim().to_string();
                            if oa_debug_count < 5 {
                                eprintln!("DEBUG OA sample row {}: nombre(C)='{}'", row_idx, nombre);
                                oa_debug_count += 1;
                            }
                            if !nombre.is_empty() {
                                let nombre_norm = normalize_name(&nombre);
                                oa_nombres.insert(nombre_norm);
                            }
                        }
                        // Si ya cargamos nombres, no hace falta intentar otras hojas
                        if !oa_nombres.is_empty() { break; }
                    }
                }
                    }
                } else {
             // Fallback: no pudimos resolver rutas automáticamente; intentamos abrir el archivo
             // de oferta usando heurística en DATAFILES_DIR como antes (no modificar comportamiento previo).
             if let Ok((_, oferta_path, _)) = crate::excel::resolve_datafile_paths(malla_archivo) {
             if let Ok(mut workbook) = calamine::open_workbook_auto(oferta_path.to_str().unwrap_or("")) {
                 let sheet_names = workbook.sheet_names().to_owned();
                 if let Some(sheet) = sheet_names.first() {
                     if let Ok(range) = workbook.worksheet_range(sheet) {
                         // contador debug para mostrar las primeras filas leídas (fallback)
                         let mut oa_debug_count_fb = 0;
                         for (row_idx, row) in range.rows().enumerate() {
                             if row_idx == 0 { continue; }
                             let oa_code_col = OA_CODE_COL.load(Ordering::Relaxed);
                             let oa_name_col = OA_NAME_COL.load(Ordering::Relaxed);
                             let codigo = data_to_string(row.get(oa_code_col).unwrap_or(&Data::Empty)).trim().to_string();
                             let nombre = data_to_string(row.get(oa_name_col).unwrap_or(&Data::Empty)).trim().to_string();
                             if oa_debug_count_fb < 5 {
                                 eprintln!("DEBUG OA(fallback) sample row {}: código='{}' | nombre='{}'", row_idx, codigo, nombre);
                                 oa_debug_count_fb += 1;
                             }
                             if !codigo.is_empty() && !nombre.is_empty() {
                                 let nombre_norm = normalize_name(&nombre);
                                 oa_nombres.insert(nombre_norm);
                             }
                         }
                     }
                 }
             }
         }
     }
    
    eprintln!("DEBUG: OA nombres cargados: {}", oa_nombres.len());
     
     // 3. Construir índice invertido: si es_electivo en PA2025-1, indexar también por codigo
     let mut porcent_by_code_electivos: HashMap<String, (String, f64, f64, bool)> = HashMap::new();
     // Iteramos sobre los valores del mapa porque la clave (nombre normalizado)
     // no se utiliza en este paso. De este modo evitamos introducir variables
     // no usadas y dejamos el código más claro.
     for entry in porcent_by_name.values() {
         let (codigo, pct, tot, es_electivo) = entry;
         if *es_electivo {
             porcent_by_code_electivos.insert(codigo.clone(), (codigo.clone(), *pct, *tot, *es_electivo));
         }
     }
     
     // 4. Recopilar todos los electivos disponibles en PA2025-1 y ordenarlos por porcentaje (DESC)
     // Los electivos con mayor porcentaje (más fáciles) se asignan primero
     let mut todos_electivos: Vec<(String, f64, f64)> = Vec::new();
     for (codigo, pct, tot, es_electivo) in porcent_by_code_electivos.values() {
         if *es_electivo {
             todos_electivos.push((codigo.clone(), *pct, *tot));
         }
     }
     // Ordenar por porcentaje DESCENDENTE (más fácil primero)
     todos_electivos.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
     eprintln!("DEBUG: {} electivos disponibles en PA2025-1 (ordenados por dificultad):", todos_electivos.len());
     for (cod, pct, _) in todos_electivos.iter() {
         eprintln!("  - {} ({}%)", cod, pct);
     }
     
     // 4. Leer Malla2020
     // Si previamente resolvimos `malla_path`, úsalo; si no, aplicar la heurística previa.
     let malla_to_open = if let Some(mp) = resolved_malla_path {
         mp
     } else {
         let resolved = if Path::new(malla_archivo).exists() {
             PathBuf::from(malla_archivo.to_string())
         } else {
             let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, malla_archivo);
             if Path::new(&candidate).exists() { PathBuf::from(candidate) } else { PathBuf::from(malla_archivo.to_string()) }
         };
         resolved
     };

    let mut workbook = open_workbook_auto(malla_to_open.to_str().unwrap_or(""))?;
    let mut ramos_disponibles = HashMap::new();
    
    // Contador para asignación secuencial de electivos sin repetir
    let mut contador_electivos = 0;
    
    // Usar hoja "Malla2020"
    let range = workbook.worksheet_range("Malla2020")?;

    // Debug: mostrar primeras filas crudas y los valores percibidos según los índices actuales
    {
        let mut dbg_count = 0usize;
        eprintln!("DEBUG: MALLA -> columnas configuradas: name={} id={}", MALLA_NAME_COL.load(Ordering::Relaxed), MALLA_ID_COL.load(Ordering::Relaxed));
        for (row_idx, row) in range.rows().enumerate() {
            if dbg_count >= 10 { break; }
            // Representación cruda de celdas
            let cells: Vec<String> = row.iter().map(|c| format!("{:?}", c)).collect();
            // Valores en las columnas configuradas (si existen)
            let name_col = MALLA_NAME_COL.load(Ordering::Relaxed);
            let id_col = MALLA_ID_COL.load(Ordering::Relaxed);
            let name_val = data_to_string(row.get(name_col).unwrap_or(&Data::Empty));
            let id_val = data_to_string(row.get(id_col).unwrap_or(&Data::Empty));
            eprintln!("DEBUG MALLA row {}: cells={:?} | name_col[{}]='{}' | id_col[{}]='{}'", row_idx, cells, name_col, name_val, id_col, id_val);
            dbg_count += 1;
        }
    }
    
    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }  // Saltar encabezado
        
        // Estructura de Malla2020: Nombre, ID, Créditos, Requisitos, Semestre, Electivo
        let malla_name_col = MALLA_NAME_COL.load(Ordering::Relaxed);
        let malla_id_col = MALLA_ID_COL.load(Ordering::Relaxed);
        let nombre = data_to_string(row.get(malla_name_col).unwrap_or(&Data::Empty)).trim().to_string();
        let id_str = data_to_string(row.get(malla_id_col).unwrap_or(&Data::Empty)).trim().to_string();
        let id = id_str.parse::<i32>().unwrap_or(0);
        
        // Leer columna Electivo (column 5)
        let es_electivo_en_malla = {
            let ev = data_to_string(row.get(5).unwrap_or(&Data::Empty)).to_lowercase();
            ev == "true" || ev == "1" || ev == "sí" || ev == "si"
        };
        
        // Leer columna Semestre (column 4) con tolerancia a formatos como "1.0", "1°", etc.
        let semestre_opt = {
            let sem_str_raw = data_to_string(row.get(4).unwrap_or(&Data::Empty)).trim().to_string();
            if sem_str_raw.is_empty() {
                None
            } else {
                // 1) intento directo entero
                sem_str_raw.parse::<i32>().ok()
                    // 2) float tipo "1.0" o con coma
                    .or_else(|| {
                        let cleaned = sem_str_raw.replace(',', '.');
                        cleaned.parse::<f64>().ok().map(|v| v.round() as i32)
                    })
                    // 3) extraer dígitos por si viene "1°" u otro sufijo
                    .or_else(|| {
                        let digits: String = sem_str_raw.chars().filter(|c| c.is_ascii_digit()).collect();
                        if digits.is_empty() { None } else { digits.parse::<i32>().ok() }
                    })
            }
        };
        
        if nombre.is_empty() || id == 0 {
            continue;
        }
        
        // DIFERENCIA CLAVE: usar estrategia diferente para electivos vs no-electivos
        let (clave_hashmap, codigo_final, dificultad, es_electivo_final) = if es_electivo_en_malla {
            // PARA ELECTIVOS: Cada ID recibe el N-ésimo electivo más fácil disponible
            // Si hay 5 electivos en Malla (IDs 44,46,50,51,52) y 10 en PA2025-1:
            // - El primer "Electivo Profesional" recibe el #1 más fácil
            // - El segundo recibe el #2 más fácil
            // - Etc., sin repetir
            
            // Contar cuántos electivos de Malla ya hemos procesado
            let indice_electivo_para_esta_id = contador_electivos;
            contador_electivos += 1;
            
            // Elegir el electivo en la posición indice_electivo_para_esta_id
            if indice_electivo_para_esta_id < todos_electivos.len() {
                let (cod_elec, pct_elec, _tot_elec) = &todos_electivos[indice_electivo_para_esta_id];
                let clave_unica = format!("electivo_profesional_{}", id);
                eprintln!("DEBUG enrich_electivo: ID={}, slot={}, asignado código='{}' ({}%)", 
                          id, indice_electivo_para_esta_id, cod_elec, pct_elec);
                (
                    clave_unica,  // CLAVE = "electivo_profesional_44", "electivo_profesional_46", etc.
                    cod_elec.clone(),  // CÓDIGO = CIT3501, CII2002, etc. (diferente para cada ID)
                    Some(*pct_elec),
                    true
                )
            } else {
                // Si hay más electivos en Malla que en PA2025-1, usar fallback
                eprintln!("WARN: No hay suficientes electivos en PA2025-1 para slot {}. Malla tiene más de {} electivos.", indice_electivo_para_esta_id, todos_electivos.len());
                let clave_unica = format!("electivo_profesional_{}", id);
                (clave_unica, id_str.clone(), None, true)
            }
        } else {
            // PARA NO-ELECTIVOS: usar nombre normalizado como clave universal
            // Estrategia simplificada: buscamos directamente por nombre normalizado en PA.
            let nombre_norm = normalize_name(&nombre);
            if let Some((codigo_encontrado, porcentaje, _total, es_electivo_en_porcent)) = porcent_by_name.get(&nombre_norm) {
                (nombre_norm, codigo_encontrado.clone(), Some(*porcentaje), *es_electivo_en_porcent)
            } else {
                // No hay match por nombre en PA: dejar código vacío y dificultad None
                (nombre_norm, String::new(), None, false)
            }
         };
         
         eprintln!("DEBUG enrich_malla: '{}' (id={}, electivo={}) → clave='{}', código='{}', dificultad={:?}", 
                   nombre, id, es_electivo_en_malla, clave_hashmap, codigo_final, dificultad);
        
        // Crear RamoDisponible enriquecido (SIN requisitos_ids aún, se resuelve en segundo pase)
        let ramo = RamoDisponible {
            id,
            nombre: nombre.clone(),
            codigo: codigo_final.clone(),
            holgura: 0,
            numb_correlativo: id,  // Correlativo es el mismo que ID
            critico: false,
            requisitos_ids: vec![],  // Se resuelve después
            dificultad,
            electivo: es_electivo_final,
            semestre: semestre_opt,  // Semestre extraído de la Malla
        };
        
        // INSERTAR CON CLAVE DIFERENCIADA (usando nombre como llave universal)
        ramos_disponibles.insert(clave_hashmap, ramo);
    }
    
    // SEGUNDO PASE: Resolver dependencias por correlativo
    // Si ramo.numb_correlativo == X, buscar ramo con numb_correlativo == X-1
    // Si existe, AGREGAR al final de requisitos_ids (no reemplazar)
    let mut updates: Vec<(String, i32)> = Vec::new();
    
    for (clave, ramo) in ramos_disponibles.iter() {
        let correlativo_actual = ramo.numb_correlativo;
        let id_anterior = correlativo_actual - 1;
        
        // Solo procesar si NO hay requisitos explícitos ya (para no sobrescribir)
        // Si ya tiene requisitos, no modificar
        if ramo.requisitos_ids.is_empty() {
            // Buscar si existe un ramo con numb_correlativo == id_anterior
            for (_, otro_ramo) in ramos_disponibles.iter() {
                if otro_ramo.numb_correlativo == id_anterior {
                    // Encontrado: el ramo anterior tiene id = id_anterior
                    updates.push((clave.clone(), id_anterior));
                    eprintln!("DEBUG depends: ramo {} (id={}) depende de ramo con id={}", 
                              ramo.nombre, correlativo_actual, id_anterior);
                    break;
                }
            }
        }
    }
    
    // Aplicar actualizaciones (solo a cursos sin requisitos explícitos)
    for (clave, id_prev) in updates {
        if let Some(ramo) = ramos_disponibles.get_mut(&clave) {
            // Solo asignar si aún no tiene requisitos
            if ramo.requisitos_ids.is_empty() {
                ramo.requisitos_ids = vec![id_prev];
            }
        }
    }
    
    Ok(ramos_disponibles)
}

// Índices por defecto (edítalos aquí si necesitas otro mapeo):
// - MALLA_NAME_COL: columna donde está el NOMBRE en la MALLA (A1 => index 0)
// - OA_NAME_COL: columna donde está el NOMBRE en la OA (C1 => index 2)
// Índices configurables (se pueden cambiar en tiempo de ejecución si se desea)


