use std::collections::HashMap;
use std::path::PathBuf;
use calamine::{open_workbook_auto, Data, Reader};
use quickshift::excel::resolve_datafile_paths;

/// Estructura para retornar información de parseo con detalles útiles
struct ParseResult {
    courses: HashMap<String, String>,
    total_rows: usize,
    header_row: usize,
    sheet_name: String,
}

/// Convierte un dato de calamine a string de forma robusta
fn data_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract().abs() < std::f64::EPSILON { 
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

/// Detecta columnas de código y nombre de forma robusta
fn detect_header_columns(row: &[Data]) -> (Option<usize>, Option<usize>) {
    let mut code_idx = None;
    let mut name_idx = None;
    
    for (col_idx, cell) in row.iter().enumerate() {
        let text = data_to_string(cell).to_lowercase();
        let trimmed = text.trim();
        
        // Detectar columna de código
        if code_idx.is_none() && (
            trimmed == "asignatura" || 
            trimmed == "codigo" || 
            trimmed == "código" || 
            trimmed == "cod" ||
            trimmed.starts_with("codigo")
        ) {
            code_idx = Some(col_idx);
        }
        
        // Detectar columna de nombre (antes de "nombre" para capturar "nombre asig.")
        if name_idx.is_none() && (
            trimmed.contains("nombre asig") ||
            trimmed == "nombre asig." ||
            trimmed == "nombre asignatura" ||
            trimmed == "nombre" ||
            trimmed == "descripcion"
        ) {
            name_idx = Some(col_idx);
        }
    }
    
    (code_idx, name_idx)
}

/// Filtra filas que no son datos reales de cursos
fn is_valid_course_code(code: &str) -> bool {
    if code.is_empty() { return false; }
    
    let lowc = code.to_lowercase();
    
    // Excluir filas que no son cursos
    if lowc.contains("sección") || 
       lowc.contains("num") || 
       lowc.contains("tipo") || 
       lowc.contains("codigo plan") || 
       lowc == "final" ||
       lowc == "total" ||
       lowc.contains("suma") {
        return false;
    }
    
    // Debe tener dígitos para ser un código de curso
    code.chars().any(|ch| ch.is_ascii_digit())
}

/// Lee cursos desde un archivo XLSX, retornando código->nombre y detalles del parseo
fn read_courses_from_xlsx(path: &PathBuf) -> Result<ParseResult, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let mut courses: HashMap<String, String> = HashMap::new();
    let mut total_rows = 0;
    let mut header_row = 0;
    let mut found_sheet = String::new();
    
    for sheet_name in workbook.sheet_names().to_owned() {
        let range = match workbook.worksheet_range(&sheet_name) { 
            Ok(r) => r, 
            Err(_) => continue 
        };
        
        // Buscar fila de encabezado en las primeras 10 filas
        let mut header_idx: Option<usize> = None;
        let mut code_idx: Option<usize> = None;
        let mut name_idx: Option<usize> = None;
        
        for (row_idx, row) in range.rows().enumerate().take(10) {
            // Salta filas completamente vacías
            if row.iter().all(|c| matches!(c, Data::Empty)) { 
                continue; 
            }
            
            let (code_col, name_col) = detect_header_columns(row);
            
            if code_col.is_some() && name_col.is_some() {
                header_idx = Some(row_idx);
                code_idx = code_col;
                name_idx = name_col;
                header_row = row_idx;
                found_sheet = sheet_name.clone();
                break;
            }
        }
        
        // Si no encontró encabezado, continuar con siguiente hoja
        let (code_col, name_col) = match (code_idx, name_idx) {
            (Some(c), Some(n)) => (c, n),
            _ => continue,
        };
        
        // Procesar filas de datos
        for (row_idx, row) in range.rows().enumerate() {
            // Skip filas vacías y encabezado
            if row.iter().all(|c| matches!(c, Data::Empty)) { 
                continue; 
            }
            if let Some(h) = header_idx {
                if row_idx == h { 
                    continue; 
                }
            }
            
            total_rows += 1;
            
            // Extraer código y nombre
            let code = row
                .get(code_col)
                .map(|c| data_to_string(c).trim().to_string())
                .unwrap_or_default();
            
            let name = row
                .get(name_col)
                .map(|c| data_to_string(c).trim().to_string())
                .unwrap_or_default();
            
            // Validar que sea un código válido
            if !is_valid_course_code(&code) {
                continue;
            }
            
            // Normalizar nombre: si está vacío o es igual al código, dejar vacío
            let name_final = if name.is_empty() || name.eq_ignore_ascii_case(&code) {
                String::new()
            } else {
                name
            };
            
            // Insertar o actualizar curso
            courses.entry(code)
                .and_modify(|existing| {
                    // Preferir nombre no-vacío
                    if existing.is_empty() && !name_final.is_empty() {
                        *existing = name_final.clone();
                    }
                })
                .or_insert(name_final);
        }
        
        // Si encontramos datos en esta hoja, no procesar más
        if !courses.is_empty() {
            break;
        }
    }
    
    Ok(ParseResult {
        courses,
        total_rows,
        header_row,
        sheet_name: found_sheet,
    })
}

#[test]
fn check_inconsistencias_oa20251() {
    eprintln!("\n🔎 TEST: Verificación de consistencia OA20251.xlsx ↔ MC2020.xlsx\n");

    // Resolver rutas de archivos
    let (malla_path_s, _, _) = resolve_datafile_paths("MC2020.xlsx")
        .expect("No se pudo resolver MC2020.xlsx");
    let malla_path = PathBuf::from(malla_path_s);
    assert!(malla_path.exists(), "Archivo MC2020.xlsx no existe: {:?}", malla_path);

    let oa_path = match resolve_datafile_paths("OA20251.xlsx") {
        Ok((p, _, _)) => PathBuf::from(p),
        Err(_) => panic!("No se encontró OA20251.xlsx en datafiles"),
    };
    assert!(oa_path.exists(), "Archivo OA20251.xlsx no existe: {:?}", oa_path);

    // Parsear archivos
    let malla_result = read_courses_from_xlsx(&malla_path)
        .expect("Error al leer MC2020.xlsx");
    let oa_result = read_courses_from_xlsx(&oa_path)
        .expect("Error al leer OA20251.xlsx");

    let malla = &malla_result.courses;
    let oa = &oa_result.courses;

    // Mostrar resumen de parseo
    eprintln!("📊 Resumen de parseo:");
    eprintln!("  MC2020.xlsx: {} filas procesadas, {} cursos únicos (encabezado en fila {})",
        malla_result.total_rows, malla.len(), malla_result.header_row);
    eprintln!("  OA20251.xlsx: {} filas procesadas, {} cursos únicos (encabezado en fila {})",
        oa_result.total_rows, oa.len(), oa_result.header_row);
    eprintln!();

    // Clasificar inconsistencias
    let mut missing_in_malla: Vec<_> = Vec::new();
    let mut missing_in_oa: Vec<_> = Vec::new();
    let mut name_mismatches: Vec<_> = Vec::new();

    // Cursos en OA que no están en Malla
    for (code, name_oa) in oa.iter() {
        match malla.get(code) {
            None => {
                missing_in_malla.push((code.clone(), name_oa.clone()));
            }
            Some(name_m) => {
                let nm_norm = name_m.to_lowercase();
                let no_norm = name_oa.to_lowercase();
                if nm_norm != no_norm && !name_m.is_empty() && !name_oa.is_empty() {
                    name_mismatches.push((code.clone(), name_m.clone(), name_oa.clone()));
                }
            }
        }
    }

    // Cursos en Malla que no están en OA
    for (code, name_m) in malla.iter() {
        if !oa.contains_key(code) {
            missing_in_oa.push((code.clone(), name_m.clone()));
        }
    }

    // Ordenar para output consistente
    missing_in_malla.sort_by(|a, b| a.0.cmp(&b.0));
    missing_in_oa.sort_by(|a, b| a.0.cmp(&b.0));
    name_mismatches.sort_by(|a, b| a.0.cmp(&b.0));

    // Validar integridad del parseo
    let total_unique = std::cmp::max(malla.len(), oa.len());
    let max_inconsistencies = std::cmp::max(missing_in_malla.len(), missing_in_oa.len());
    
    if max_inconsistencies > total_unique {
        eprintln!("⚠️  ADVERTENCIA: Posible fallo de parseo (inconsistencias > ramos únicos)");
        eprintln!("   Inconsistencias: {} > {} ramos únicos", max_inconsistencies, total_unique);
    }

    // Reportar resultados
    if missing_in_malla.is_empty() && missing_in_oa.is_empty() && name_mismatches.is_empty() {
        eprintln!("✅ ÉXITO: No se encontraron inconsistencias\n");
        return;
    }

    eprintln!("❌ Se encontraron inconsistencias:\n");

    if !missing_in_malla.is_empty() {
        eprintln!("📌 {} cursos en OA20251 pero NO en MC2020:", missing_in_malla.len());
        for (code, name) in missing_in_malla.iter().take(20) {
            eprintln!("   • {:<12} {}", code, name);
        }
        if missing_in_malla.len() > 20 {
            eprintln!("   ... y {} más", missing_in_malla.len() - 20);
        }
        eprintln!();
    }

    if !missing_in_oa.is_empty() {
        eprintln!("📌 {} cursos en MC2020 pero NO en OA20251:", missing_in_oa.len());
        for (code, name) in missing_in_oa.iter().take(20) {
            eprintln!("   • {:<12} {}", code, name);
        }
        if missing_in_oa.len() > 20 {
            eprintln!("   ... y {} más", missing_in_oa.len() - 20);
        }
        eprintln!();
    }

    if !name_mismatches.is_empty() {
        eprintln!("📌 {} cursos con nombre distinto:", name_mismatches.len());
        for (code, name_m, name_oa) in name_mismatches.iter().take(10) {
            eprintln!("   • {}:", code);
            eprintln!("     MC2020:  '{}'", name_m);
            eprintln!("     OA20251: '{}'", name_oa);
        }
        if name_mismatches.len() > 10 {
            eprintln!("   ... y {} más", name_mismatches.len() - 10);
        }
        eprintln!();
    }

    eprintln!("💡 Resumen: {} en OA, {} en MC, {} discrepancias",
        oa.len(), malla.len(),
        missing_in_malla.len() + missing_in_oa.len() + name_mismatches.len());

    panic!("Test fallido: existen inconsistencias entre archivos. Revisa el reporte anterior.");
}
