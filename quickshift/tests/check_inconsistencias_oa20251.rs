use std::collections::HashMap;
use std::path::PathBuf;
use calamine::{open_workbook_auto, Data, Reader};
use quickshift::excel::resolve_datafile_paths;

fn debug_print_sheet_headers(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    eprintln!("\n--- DEBUG: Inspección rápida de hojas para: {:?} ---", path);
    for sheet_name in workbook.sheet_names().to_owned() {
        eprintln!("Hoja: {}", sheet_name);
        let range = match workbook.worksheet_range(&sheet_name) { Ok(r) => r, Err(_) => continue };
        for (ridx, row) in range.rows().enumerate().take(10) {
            let cells: Vec<String> = row.iter().map(|c| data_to_string(c)).collect();
            eprintln!("  fila {} -> {:?}", ridx, cells);
        }
        eprintln!("\n");
    }
    Ok(())
}

fn data_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => "".to_string(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if (f.fract().abs() - 0.0).abs() < std::f64::EPSILON { format!("{}", *f as i64) } else { f.to_string() }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => format!("{}", b),
        _ => format!("{:?}", cell),
    }
}

fn read_courses_from_xlsx(path: &PathBuf) -> Result<(HashMap<String, String>, usize), Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let mut map: HashMap<String, String> = HashMap::new();
    let mut total_row_count: usize = 0;
    
    for sheet_name in workbook.sheet_names().to_owned() {
        let range = match workbook.worksheet_range(&sheet_name) { Ok(r) => r, Err(_) => continue };
        
        // PASO 1: Detectar encabezado y columnas de código/nombre
        let mut header_row_idx: Option<usize> = None;
        let mut code_idx: Option<usize> = None;
        let mut name_idx: Option<usize> = None;
        
        for (ridx, row) in range.rows().enumerate() {
            if ridx > 5 { break; } // Search only in first few rows
            
            let mut has_codigo = false;
            let mut has_nombre = false;
            let mut code_col: Option<usize> = None;
            let mut name_col: Option<usize> = None;
            
            for (ci, cell) in row.iter().enumerate() {
                let txt = data_to_string(cell).to_lowercase();
                let ttrim = txt.trim();
                
                // Detectar columna de código: "asignatura", "codigo", "código", "cod"
                if !has_codigo && (ttrim == "asignatura" || ttrim == "codigo" || ttrim == "código" || ttrim == "cod") {
                    has_codigo = true;
                    code_col = Some(ci);
                }
                
                // Detectar columna de nombre: "nombre asig", "nombre", "nombre asignatura", "descripcion"
                // IMPORTANTE: "nombre asig" antes que "nombre" para capturar "Nombre Asig." correctamente
                if !has_nombre && (ttrim.contains("nombre asig") || ttrim == "nombre asig." || ttrim == "nombre" || ttrim == "nombre asignatura") {
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
        
        // PASO 2: Procesar filas de datos
        for (row_idx, row) in range.rows().enumerate() {
            if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }
            
            // Skip header row
            if let Some(h) = header_row_idx {
                if row_idx == h { continue; }
            }
            
            total_row_count += 1;
            
            // Extraer código y nombre de las columnas detectadas
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
            
            // Validaciones básicas
            if code.is_empty() { continue; }
            
            let lowc = code.to_lowercase();
            if lowc.contains("sección") || lowc.contains("num") || lowc.contains("tipo") || 
               lowc.contains("codigo plan") || lowc == "final" { 
                continue; 
            }
            
            // Validar que el código tenga la forma de un código (contiene dígitos)
            if !code.chars().any(|ch| ch.is_ascii_digit()) {
                continue;
            }
            
            // Si el nombre está vacío o es igual al código, usar vacío (no hacer fallback a otras columnas)
            let name_final = if name.trim().is_empty() || name.trim().eq_ignore_ascii_case(&code.trim()) {
                "".to_string()
            } else {
                name
            };
            
            match map.get_mut(&code) {
                Some(existing) => {
                    if existing.trim().is_empty() && !name_final.trim().is_empty() {
                        *existing = name_final;
                    }
                }
                None => { 
                    map.insert(code, name_final); 
                }
            }
        }
    }
    
    Ok((map, total_row_count))
}

#[test]
fn check_inconsistencias_oa20251() {
    eprintln!("\n🔎 TEST: Inconsistencias OA20251 vs MC2020.xlsx (agrupadas por ramo)");

    let (malla_path_s, _oferta, _p) = resolve_datafile_paths("MC2020.xlsx").expect("No se pudo resolver MC2020.xlsx");
    let malla_path = PathBuf::from(malla_path_s);
    assert!(malla_path.exists(), "Archivo MC2020.xlsx no existe: {:?}", malla_path);

    // prefer OA20251.xlsx specifically
    let oa_candidate = "OA20251.xlsx";
    let oa_path = match resolve_datafile_paths(oa_candidate) {
        Ok((p, _, _)) => PathBuf::from(p),
        Err(_) => panic!("No se encontró {} en datafiles", oa_candidate),
    };
    assert!(oa_path.exists(), "Archivo OA20251.xlsx no existe: {:?}", oa_path);

    let (malla, malla_rows) = read_courses_from_xlsx(&malla_path).expect("Lectura MC2020 falló");
    // debug-print the first 10 rows of the OA workbook to inspect headers/cells
    debug_print_sheet_headers(&oa_path).expect("Debug print failed");
    let (oa, oa_rows) = read_courses_from_xlsx(&oa_path).expect("Lectura OA20251 falló");

    // Grouped by course (code)
    let mut missing_in_malla: HashMap<String, String> = HashMap::new();
    let mut missing_in_oa: HashMap<String, String> = HashMap::new();
    let mut name_mismatches: HashMap<String, (String, String)> = HashMap::new();

    for (code, name_oa) in oa.iter() {
        match malla.get(code) {
            None => { missing_in_malla.insert(code.clone(), name_oa.clone()); }
            Some(name_m) => {
                let nm = name_m.trim().to_lowercase();
                let no = name_oa.trim().to_lowercase();
                if nm != no {
                    name_mismatches.insert(code.clone(), (name_m.clone(), name_oa.clone()));
                }
            }
        }
    }

    for (code, name_m) in malla.iter() {
        if !oa.contains_key(code) {
            missing_in_oa.insert(code.clone(), name_m.clone());
        }
    }

    // Sanity checks: print parsed counts and rows; tests should fail if counts indicate a parsing failure
    eprintln!("MC2020.xlsx: filas_no_vacias={}  ramos_parseados={}", malla_rows, malla.len());
    eprintln!("OA20251.xlsx: filas_no_vacias={}  ramos_parseados={}", oa_rows, oa.len());

    // Use parsed (unique) code counts as the row count for comparison — rows correspond to ramos not sections.
    let max_parsed = std::cmp::max(malla.len(), oa.len());
    if missing_in_oa.len() > max_parsed {
        panic!("Fallo de lectura: demasiados ramos de MC2020 faltantes en OA ({} faltantes > ramos max {}); revisa el parser.", missing_in_oa.len(), max_parsed);
    }
    if missing_in_malla.len() > max_parsed {
        panic!("Fallo de lectura: demasiados ramos de OA faltantes en MC2020 ({} faltantes > ramos max {}); revisa el parser.", missing_in_malla.len(), max_parsed);
    }

    if missing_in_malla.is_empty() && missing_in_oa.is_empty() && name_mismatches.is_empty() {
        eprintln!("✅ No se encontraron inconsistencias entre OA20251.xlsx y MC2020.xlsx");
        return;
    }

    eprintln!("\n❌ Se encontraron inconsistencias entre OA20251.xlsx y MC2020.xlsx:\n");

    if !missing_in_malla.is_empty() {
        eprintln!("👉 Cursos presentes en OA20251.xlsx pero NO en MC2020.xlsx (posible no_en_malla):");
        for (code, name) in missing_in_malla.iter() {
            eprintln!("   - {}: {}", code, name);
        }
        eprintln!("");
    }

    if !missing_in_oa.is_empty() {
        eprintln!("👉 Cursos presentes en MC2020.xlsx pero NO en OA20251.xlsx (posible falta de mapeo):");
        for (code, name) in missing_in_oa.iter() {
            eprintln!("   - {}: {}", code, name);
        }
        eprintln!("");
    }

    if !name_mismatches.is_empty() {
        eprintln!("👉 Cursos con nombre distinto para mismo código:");
        for (code, (name_m, name_oa)) in name_mismatches.iter() {
            eprintln!("   - {}: MC2020='{}'  <--->  OA20251='{}'", code, name_m, name_oa);
        }
        eprintln!("");
    }

    panic!("Inconsistencias detectadas entre OA20251.xlsx y MC2020.xlsx. Revisa el reporte anterior.");
}
