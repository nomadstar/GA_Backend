use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;
use calamine::{open_workbook_auto, Data, Reader};
use quickshift::excel::resolve_datafile_paths;

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
fn dump_malla_and_oa_parsed_to_csv() {
    eprintln!("🔎 Generando CSV de MC2020.xlsx y OA20251.xlsx para inspección");
    let (malla_path_s, _oferta, _p) = resolve_datafile_paths("MC2020.xlsx").expect("No se pudo resolver MC2020.xlsx");
    let malla_path = PathBuf::from(malla_path_s);
    let (oa_path_s, _oferta2, _p2) = resolve_datafile_paths("OA20251.xlsx").expect("No se pudo resolver OA20251.xlsx");
    let oa_path = PathBuf::from(oa_path_s);

    let (malla, _mr) = read_courses_from_xlsx(&malla_path).expect("Lectura MC2020 falló");
    let (oa, _or) = read_courses_from_xlsx(&oa_path).expect("Lectura OA20251 falló");

    // Write CSVs
    let mut f_m = File::create("/tmp/malla_parsed.csv").expect("No se pudo crear /tmp/malla_parsed.csv");
    writeln!(f_m, "codigo,nombre").unwrap();
    for (c, n) in malla.iter() { writeln!(f_m, "{},{}", c, n.replace(',', " ")).ok(); }

    let mut f_o = File::create("/tmp/oa20251_parsed.csv").expect("No se pudo crear /tmp/oa20251_parsed.csv");
    writeln!(f_o, "codigo,nombre").unwrap();
    for (c, n) in oa.iter() { writeln!(f_o, "{},{}", c, n.replace(',', " ")).ok(); }

    // Generate mismatch CSV
    let mut f_i = File::create("/tmp/inconsistencias_mapa.csv").expect("No se pudo crear /tmp/inconsistencias_mapa.csv");
    writeln!(f_i, "codigo,in_malla,in_oa,nombre_malla,nombre_oa").unwrap();
    let mut keys: Vec<String> = malla.keys().chain(oa.keys()).map(|s| s.clone()).collect();
    keys.sort(); keys.dedup();
    for k in keys.iter() {
        let in_m = malla.contains_key(k);
        let in_o = oa.contains_key(k);
        let nm = malla.get(k).map(|s| s.replace(',', " ")).unwrap_or_default();
        let no = oa.get(k).map(|s| s.replace(',', " ")).unwrap_or_default();
        writeln!(f_i, "{},{},{},{},{}", k, in_m, in_o, nm, no).unwrap();
    }

    eprintln!("CSVs escritos:\n - /tmp/malla_parsed.csv\n - /tmp/oa20251_parsed.csv\n - /tmp/inconsistencias_mapa.csv");

    // Additionally, gather candidate names per code from OA by scanning columns that may contain descriptive names
    let mut wb = open_workbook_auto(&oa_path).expect("open oa");
    use std::collections::BTreeSet;
    let mut candidates: HashMap<String, BTreeSet<String>> = HashMap::new();
    for sheet_name in wb.sheet_names().to_owned() {
        let range = match wb.worksheet_range(&sheet_name) { Ok(r) => r, Err(_) => continue };
        for row in range.rows() {
            if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }
            // first non-empty cell as code candidate
            let code_opt = row.iter().map(|c| data_to_string(c).trim().to_string()).find(|s| !s.is_empty());
            if let Some(code) = code_opt {
                // collect possible name candidates from column 1 (Nombre Asig.) and 5 (Descrip. Evento) if present
                let mut set = candidates.entry(code.clone()).or_insert_with(BTreeSet::new);
                if row.len() > 1 { let s = data_to_string(&row[1]).trim().to_string(); if !s.is_empty() && !s.eq_ignore_ascii_case(&code) { set.insert(s); } }
                if row.len() > 5 { let s = data_to_string(&row[5]).trim().to_string(); if !s.is_empty() && !s.eq_ignore_ascii_case(&code) { set.insert(s); } }
            }
        }
    }
    let mut f_c = File::create("/tmp/oa_name_candidates.csv").expect("No se pudo crear /tmp/oa_name_candidates.csv");
    writeln!(f_c, "codigo,candidate_names").unwrap();
    let mut keys: Vec<String> = candidates.keys().cloned().collect(); keys.sort();
    for k in keys.iter() {
        let joined = candidates.get(k).map(|s| s.iter().cloned().collect::<Vec<_>>().join(" ; ")).unwrap_or_default();
        writeln!(f_c, "{},{}", k, joined.replace(',', " ")).unwrap();
    }
    eprintln!("Adicional: /tmp/oa_name_candidates.csv");
}
