use std::collections::HashMap;
use std::path::PathBuf;
use calamine::{open_workbook_auto, Data, Reader};
use strsim::jaro_winkler;
use quickshift::excel::resolve_datafile_paths;

fn data_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => "".to_string(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if (f.fract().abs() - 0.0).abs() < std::f64::EPSILON { 
                format!("{}", *f as i64) 
            } else { 
                f.to_string() 
            }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => format!("{}", b),
        _ => format!("{:?}", cell),
    }
}

fn normalize_name(s: &str) -> String {
    s.to_lowercase()
        .trim()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Lee OA20251 y devuelve HashMap<nombre_normalizado, código>
fn read_oa20251_codes(path: &PathBuf) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let mut map: HashMap<String, String> = HashMap::new();
    
    for sheet_name in workbook.sheet_names().to_owned() {
        let range = match workbook.worksheet_range(&sheet_name) { Ok(r) => r, Err(_) => continue };
        
        // Detectar encabezado
        let mut header_row_idx: Option<usize> = None;
        let mut code_idx: Option<usize> = None;
        let mut name_idx: Option<usize> = None;
        
        for (ridx, row) in range.rows().enumerate() {
            if ridx > 5 { break; }
            
            let mut has_codigo = false;
            let mut has_nombre = false;
            let mut code_col: Option<usize> = None;
            let mut name_col: Option<usize> = None;
            
            for (ci, cell) in row.iter().enumerate() {
                let txt = data_to_string(cell).to_lowercase();
                let ttrim = txt.trim();
                
                if !has_codigo && (ttrim == "asignatura" || ttrim == "codigo" || ttrim == "código" || ttrim == "asig") {
                    has_codigo = true;
                    code_col = Some(ci);
                }
                
                if !has_nombre && (txt.contains("nombre asig") || ttrim.contains("nombre asig.") || txt.contains("nombre")) {
                    has_nombre = true;
                    name_col = Some(ci);
                }
            }
            
            if has_codigo && has_nombre {
                header_row_idx = Some(ridx);
                code_idx = code_col;
                name_idx = name_col;
                break;
            }
        }
        
        // Leer filas de datos
        for (row_idx, row) in range.rows().enumerate() {
            if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }
            
            if let Some(h) = header_row_idx {
                if row_idx == h { continue; }
            }
            
            let code = if let Some(cidx) = code_idx {
                row.get(cidx).map(|c| data_to_string(c).trim().to_string()).unwrap_or_default()
            } else {
                "".to_string()
            };
            
            let name = if let Some(nidx) = name_idx {
                row.get(nidx).map(|c| data_to_string(c).trim().to_string()).unwrap_or_default()
            } else {
                "".to_string()
            };
            
            if code.is_empty() || name.is_empty() { continue; }
            if !code.chars().any(|ch| ch.is_ascii_digit()) { continue; }
            
            let name_norm = normalize_name(&name);
            if !map.contains_key(&name_norm) {
                map.insert(name_norm, code);
            }
        }
    }
    
    Ok(map)
}

/// Lee MC2020 y devuelve estructura para modificar
fn read_mc2020_structure(path: &PathBuf) -> Result<(Vec<Vec<String>>, Vec<String>), Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let range = match workbook.worksheet_range("MallaCurricular2020") { Ok(r) => r, Err(_) => return Err("No se encontró hoja MallaCurricular2020".into()) };
    
    let mut header: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    
    for (ridx, row) in range.rows().enumerate() {
        let row_strings: Vec<String> = row.iter().map(|c| data_to_string(c)).collect();
        if ridx == 0 {
            header = row_strings;
        } else {
            if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }
            rows.push(row_strings);
        }
    }
    
    Ok((rows, header))
}

#[test]
fn generate_malla_with_oa_codes() {
    eprintln!("\n🔄 TEST: Generar MC2020 con códigos corregidos de OA20251");
    eprintln!("============================================================\n");
    
    let (oa_path_s, _, _) = resolve_datafile_paths("OA20251.xlsx").expect("OA20251 no encontrado");
    let oa_path = PathBuf::from(oa_path_s);
    
    let (mc_path_s, _, _) = resolve_datafile_paths("MC2020.xlsx").expect("MC2020 no encontrado");
    let mc_path = PathBuf::from(mc_path_s);
    
    // Paso 1: Leer códigos de OA20251
    eprintln!("📖 PASO 1: Leyendo códigos de OA20251.xlsx");
    let oa_codes = read_oa20251_codes(&oa_path).expect("Error al leer OA20251");
    eprintln!("✅ {} cursos cargados desde OA20251", oa_codes.len());
    
    // Paso 2: Leer estructura de MC2020
    eprintln!("\n📖 PASO 2: Leyendo estructura de MC2020.xlsx");
    let (mc_rows, header) = read_mc2020_structure(&mc_path).expect("Error al leer MC2020");
    eprintln!("✅ {} cursos cargados desde MC2020", mc_rows.len());
    
    // Encontrar columnas de ID y Nombre
    let id_col = header.iter().position(|h| h.to_lowercase().contains("id") || h.to_lowercase() == "id").unwrap_or(1);
    let name_col = header.iter().position(|h| h.to_lowercase().contains("nombre") && !h.to_lowercase().contains("id")).unwrap_or(0);
    
    eprintln!("\n📋 Estructura detectada:");
    eprintln!("  Columna de ID: {} ({:?})", id_col, header.get(id_col));
    eprintln!("  Columna de Nombre: {} ({:?})", name_col, header.get(name_col));
    
    // Paso 3: Matchear por nombre con threshold 0.7
    eprintln!("\n🔍 PASO 3: Matcheando cursos (threshold=0.7)");
    const THRESHOLD: f64 = 0.7;
    
    let mut matches_found = 0;
    let mut matches_details: Vec<(String, String, String, f64)> = Vec::new();
    
    for (_row_idx, row) in mc_rows.iter().enumerate() {
        if row.len() <= name_col { continue; }
        
        let mc_name = &row[name_col];
        let mc_id = row.get(id_col).map(|s| s.as_str()).unwrap_or("");
        
        if mc_name.trim().is_empty() { continue; }
        
        let mc_name_norm = normalize_name(mc_name);
        
        // Buscar best match en OA20251
        let mut best_match: Option<(String, f64)> = None;
        
        for (oa_name, oa_code) in oa_codes.iter() {
            let similarity = jaro_winkler(&mc_name_norm, oa_name);
            
            if similarity >= THRESHOLD {
                if let Some((_, prev_sim)) = &best_match {
                    if similarity > *prev_sim {
                        best_match = Some((oa_code.clone(), similarity));
                    }
                } else {
                    best_match = Some((oa_code.clone(), similarity));
                }
            }
        }
        
        if let Some((oa_code, similarity)) = best_match {
            matches_found += 1;
            matches_details.push((mc_id.to_string(), mc_name.clone(), oa_code.clone(), similarity));
            eprintln!("  ✓ {} -> {} (similitud: {:.1}%)", mc_id, oa_code, similarity * 100.0);
        }
    }
    
    eprintln!("\n✅ {} cursos con match exitoso (threshold ≥ 70%)", matches_found);
    
    // Paso 4: Crear MC2020 corregido
    eprintln!("\n📝 PASO 4: Generando MC2020 con códigos corregidos");
    
    let mut corrected_rows = mc_rows.clone();
    let mut corrections_applied = 0;
    
    for (mc_id, _, oa_code, _) in &matches_details {
        // Encontrar la fila con este ID y actualizar
        for row in corrected_rows.iter_mut() {
            if row.len() > id_col && row[id_col] == *mc_id {
                if row.len() > id_col {
                    row[id_col] = oa_code.clone();
                    corrections_applied += 1;
                }
                break;
            }
        }
    }
    
    eprintln!("✅ {} código(s) corregido(s)", corrections_applied);
    
    // Paso 5: Guardar archivo corregido
    eprintln!("\n💾 PASO 5: Guardando resultados");
    
    use std::fs::File;
    use std::io::Write;
    
    // Guardar CSV para análisis rápido
    let output_path_csv = "/tmp/MC2020_corregido_mapping.csv";
    let mut output_file = File::create(output_path_csv).expect("No se pudo crear archivo CSV");
    
    // Escribir encabezado
    writeln!(output_file, "{}", header.join(",")).ok();
    
    // Escribir filas corregidas
    for row in &corrected_rows {
        writeln!(output_file, "{}", row.join(",")).ok();
    }
    
    eprintln!("✅ CSV guardado en: {}", output_path_csv);
    
    // Guardar también JSON con mapeo detallado
    let json_mapping: Vec<serde_json::Value> = matches_details
        .iter()
        .map(|(mc_id, mc_name, oa_code, similarity)| {
            serde_json::json!({
                "mc_id": mc_id,
                "mc_name": mc_name,
                "oa_code": oa_code,
                "similarity": format!("{:.1}%", similarity * 100.0)
            })
        })
        .collect();
    
    let output_path_json = "/tmp/MC2020_OA20251_mapping.json";
    let json_str = serde_json::to_string_pretty(&json_mapping).expect("Error serializando JSON");
    let mut json_file = File::create(output_path_json).expect("No se pudo crear JSON");
    write!(json_file, "{}", json_str).ok();
    eprintln!("✅ JSON guardado en: {}", output_path_json);
    
    // Resumen
    eprintln!("\n📊 RESUMEN:");
    eprintln!("  Cursos MC2020 originales: {}", mc_rows.len());
    eprintln!("  Cursos OA20251 disponibles: {}", oa_codes.len());
    eprintln!("  Matches encontrados (≥70%): {}", matches_found);
    eprintln!("  Códigos corregidos: {}", corrections_applied);
    eprintln!("  Tasa de mapeo: {:.1}%", (matches_found as f64 / mc_rows.len() as f64) * 100.0);
    
    // Mostrar algunos matches
    if !matches_details.is_empty() {
        eprintln!("\n📋 Primeros 10 matches:");
        for (mc_id, mc_name, oa_code, similarity) in matches_details.iter().take(10) {
            eprintln!("  {} | {} -> {} | {:.1}%", mc_id, mc_name, oa_code, similarity * 100.0);
        }
    }
    
    // Mostrar algunos sin match
    let unmatched: Vec<_> = mc_rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            if row.len() <= name_col { return true; }
            let mc_id = row.get(id_col).map(|s| s.as_str()).unwrap_or("");
            !matches_details.iter().any(|(id, _, _, _)| id == mc_id)
        })
        .collect();
    
    if !unmatched.is_empty() {
        eprintln!("\n❌ Cursos sin match (< 70% similitud): {}", unmatched.len());
        for (_, row) in unmatched.iter().take(5) {
            if row.len() > name_col {
                eprintln!("  - {} | {}", row.get(id_col).unwrap_or(&"?".to_string()), &row[name_col]);
            }
        }
        if unmatched.len() > 5 {
            eprintln!("  ... y {} más", unmatched.len() - 5);
        }
    }
}

fn num_to_excel_col(num: usize) -> String {
    let mut num = num;
    let mut col = String::new();
    while num > 0 {
        num -= 1;
        col.insert(0, (b'A' + (num % 26) as u8) as char);
        num /= 26;
    }
    col
}
