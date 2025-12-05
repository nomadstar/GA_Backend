use calamine::{open_workbook_auto, Data, Reader};
use crate::models::Seccion;
use crate::excel::io::{data_to_string, read_sheet_via_zip};
use zip;
use std::collections::HashMap;

// Extrae el código base de un código de asignatura eliminando sufijos de evento
// Ej: "CBF1000_LA01" -> "CBF1000"
fn base_course_code(code: &str) -> String {
    let s = code.trim();
    if s.is_empty() { return String::new(); }
    // Separar por guión bajo '_' (caso más común en códigos de eventos)
    let first = s.split('_').next().unwrap_or(s).to_string();
    // Eliminar caracteres finales no alfanuméricos
    let mut trimmed = first;
    while let Some(ch) = trimmed.chars().last() {
        if !ch.is_ascii_alphanumeric() {
            trimmed.pop();
        } else {
            break;
        }
    }
    trimmed
}

/// Lee la oferta académica y devuelve una lista de `Seccion`.
pub fn leer_oferta_academica_excel(nombre_archivo: &str) -> Result<Vec<Seccion>, Box<dyn std::error::Error>> {
    // Resolver ruta hacia el directorio protegido `DATAFILES_DIR` si es necesario
    let resolved = if std::path::Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        // 🆕 Usar get_datafiles_dir() para runtime path resolution
        let data_dir = crate::excel::get_datafiles_dir();
        let candidate = data_dir.join(nombre_archivo);
        if candidate.exists() {
            candidate.to_string_lossy().to_string()
        } else {
            nombre_archivo.to_string()
        }
    };

    // Recolectaremos filas crudas y luego las agruparemos por (codigo, seccion, codigo_box)
    struct RawRow { codigo: String, nombre: String, seccion: String, horario: Vec<String>, profesor: String, codigo_box: String }
    let mut raw_rows: Vec<RawRow> = Vec::new();

    // Intentar primero con calamine (más rápido si funciona)
    if let Ok(mut workbook) = open_workbook_auto(&resolved) {
        let sheet_names = workbook.sheet_names().to_owned();
        
        for sheet in sheet_names.iter() {
            if let Ok(range) = workbook.worksheet_range(sheet) {
                for (row_idx, row) in range.rows().enumerate() {
                    if row_idx == 0 { continue; }  // skip header
                    if row.is_empty() { continue; }
                    
                    // Para OA2024: Columna 2 = Codigo, Columna 3 = Nombre, Columna 4 = Sección
                    let codigo = data_to_string(row.get(1).unwrap_or(&Data::Empty)).trim().to_string();
                    let base_codigo = base_course_code(&codigo);
                    if codigo.is_empty() { continue; }
                    
                    let nombre = data_to_string(row.get(2).unwrap_or(&Data::Empty)).trim().to_string();
                    let seccion = data_to_string(row.get(3).unwrap_or(&Data::Empty)).trim().to_string();
                    let horario_str = data_to_string(row.get(7).unwrap_or(&Data::Empty)).trim().to_string();
                    let profesor = data_to_string(row.get(9).unwrap_or(&Data::Empty)).trim().to_string();
                    
                    // codigo_box es el ID del paquete de clases
                    let codigo_box = data_to_string(row.get(18).unwrap_or(&Data::Empty)).trim().to_string();
                    let codigo_box = if codigo_box.is_empty() { 
                        codigo.clone() 
                    } else { 
                        codigo_box 
                    };
                    
                    let horario: Vec<String> = if horario_str.is_empty() { 
                        vec!["Sin horario".to_string()] 
                    } else { 
                        horario_str.split(|c| c == ',' || c == ';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect() 
                    };

                    raw_rows.push(RawRow { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
                }
                // Agrupar y construir secciones si recolectamos filas
                if !raw_rows.is_empty() {
                    let mut map: HashMap<(String,String,String), Vec<RawRow>> = HashMap::new();
                    for r in raw_rows.into_iter() {
                        let key = (base_course_code(&r.codigo), r.seccion.clone(), r.codigo_box.clone());
                        map.entry(key).or_insert_with(Vec::new).push(r);
                    }
                    let mut result: Vec<Seccion> = Vec::new();
                    for ((codigo, _secc, codigo_box), rows) in map.into_iter() {
                        // unir horarios y deduplicar
                        let mut horarios_acc: Vec<String> = Vec::new();
                        let mut profesor_pref = String::new();
                        let mut nombre_pref = String::new();
                        for r in rows.into_iter() {
                            if nombre_pref.is_empty() { nombre_pref = r.nombre.clone(); }
                            if profesor_pref.is_empty() && !r.profesor.trim().is_empty() { profesor_pref = r.profesor.clone(); }
                            for h in r.horario.into_iter() {
                                if !horarios_acc.iter().any(|x| x == &h) {
                                    horarios_acc.push(h);
                                }
                            }
                        }
                        if horarios_acc.is_empty() { horarios_acc.push("Sin horario".to_string()); }
                        result.push(Seccion { codigo: codigo.clone(), nombre: nombre_pref.clone(), seccion: _secc.clone(), horario: horarios_acc, profesor: profesor_pref.clone(), codigo_box: codigo_box.clone() });
                    }
                    return Ok(result);
                }
            }
        }
    }

    // Fallback: usar zip reader como alternativa si calamine falló
    eprintln!("DEBUG: calamine falló o no devolvió datos, intentando leer vía zip para '{}'", resolved);
    
    // Obtener lista de hojas desde el archivo zip
    if let Ok(archive) = zip::ZipArchive::new(std::fs::File::open(&resolved)?) {
        let file_list: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();

        for fname in file_list.iter() {
            if !fname.starts_with("xl/worksheets/sheet") { continue; }

            if let Ok(rows_vec) = read_sheet_via_zip(&resolved, fname) {
                let mut raw_rows_zip: Vec<RawRow> = Vec::new();
                for (row_idx, row) in rows_vec.iter().enumerate() {
                    if row_idx == 0 { continue; }  // skip header
                    if row.iter().all(|c| c.trim().is_empty()) { continue; }

                    // Para OA2024: Columna 2 = Codigo, Columna 3 = Nombre, Columna 4 = Sección
                    let codigo = row.get(1).cloned().unwrap_or_default().trim().to_string();
                    let base_codigo = base_course_code(&codigo);
                    if codigo.is_empty() { continue; }

                    let nombre = row.get(2).cloned().unwrap_or_else(|| "Sin nombre".to_string());
                    let seccion = row.get(3).cloned().unwrap_or_else(|| "1".to_string());
                    let horario_str = row.get(7).cloned().unwrap_or_default();
                    let profesor = row.get(9).cloned().unwrap_or_else(|| "Sin asignar".to_string());
                    let codigo_box = row.get(18).cloned().unwrap_or_else(|| codigo.clone());

                    let horario: Vec<String> = if horario_str.is_empty() { 
                        vec!["Sin horario".to_string()] 
                    } else { 
                        horario_str.split(|c| c == ',' || c == ';')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect() 
                    };

                    raw_rows_zip.push(RawRow { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
                }

                if !raw_rows_zip.is_empty() {
                    let mut map: HashMap<(String,String,String), Vec<RawRow>> = HashMap::new();
                    for r in raw_rows_zip.into_iter() {
                        let key = (base_course_code(&r.codigo), r.seccion.clone(), r.codigo_box.clone());
                        map.entry(key).or_insert_with(Vec::new).push(r);
                    }
                    let mut result: Vec<Seccion> = Vec::new();
                    for ((codigo, secc, codigo_box), rows) in map.into_iter() {
                        let mut horarios_acc: Vec<String> = Vec::new();
                        let mut profesor_pref = String::new();
                        let mut nombre_pref = String::new();
                        for r in rows.into_iter() {
                            if nombre_pref.is_empty() { nombre_pref = r.nombre.clone(); }
                            if profesor_pref.is_empty() && !r.profesor.trim().is_empty() { profesor_pref = r.profesor.clone(); }
                            for h in r.horario.into_iter() {
                                if !horarios_acc.iter().any(|x| x == &h) {
                                    horarios_acc.push(h);
                                }
                            }
                        }
                        if horarios_acc.is_empty() { horarios_acc.push("Sin horario".to_string()); }
                        result.push(Seccion { codigo: codigo.clone(), nombre: nombre_pref.clone(), seccion: secc.clone(), horario: horarios_acc, profesor: profesor_pref.clone(), codigo_box: codigo_box.clone() });
                    }
                    eprintln!("DEBUG: leer_oferta_academica_excel cargó {} secciones vía zip agrupadas", result.len());
                    return Ok(result);
                }
            }
        }
    }

    Err(format!("No se pudo leer ninguna hoja del archivo '{}'.", nombre_archivo).into())
}

/// Genera un resumen de la oferta académica: nombre del ramo → cantidad de secciones
pub fn resumen_oferta_academica(nombre_archivo: &str) -> Result<Vec<(String, usize)>, Box<dyn std::error::Error>> {
    let secciones = leer_oferta_academica_excel(nombre_archivo)?;
    
    let mut resumen: HashMap<String, usize> = HashMap::new();
    
    for seccion in secciones.iter() {
        *resumen.entry(seccion.nombre.clone()).or_insert(0) += 1;
    }
    
    let mut result: Vec<(String, usize)> = resumen.into_iter().collect();
    result.sort_by(|a, b| {
        // Ordenar por número de secciones descendente, luego por nombre
        match b.1.cmp(&a.1) {
            std::cmp::Ordering::Equal => a.0.cmp(&b.0),
            other => other,
        }
    });
    
    Ok(result)
}
