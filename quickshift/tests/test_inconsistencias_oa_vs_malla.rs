use std::collections::HashMap;
use std::path::PathBuf;
use calamine::{open_workbook_auto, Data, Reader};

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

fn read_courses_from_xlsx(path: &PathBuf) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let mut map: HashMap<String, String> = HashMap::new();

    for sheet_name in workbook.sheet_names().to_owned() {
        let range = match workbook.worksheet_range(&sheet_name) {
            Ok(r) => r,
            Err(_) => continue,
        };

        // Try to detect header row and indices for code/name columns
        let mut header_row_idx: Option<usize> = None;
        let mut code_idx: Option<usize> = None;
        let mut name_idx: Option<usize> = None;

        for (ridx, row) in range.rows().enumerate() {
            let row_texts: Vec<String> = row.iter().map(|c| data_to_string(c).to_lowercase()).collect();
            let has_codigo = row_texts.iter().any(|s| s.contains("codigo") || s.contains("código") || s.contains("cod"));
            let has_nombre = row_texts.iter().any(|s| s.contains("nombre") || s.contains("asignatura") || s.contains("descripcion"));
            let has_seccion = row_texts.iter().any(|s| s.contains("sección") || s.contains("seccion"));

            if (has_codigo && has_nombre) || (has_seccion && has_nombre) {
                header_row_idx = Some(ridx);
                // Prefer an exact/simple "codigo" column if present (avoids picking "Código Plan Estudio")
                for (ci, cell) in row.iter().enumerate() {
                    let txt = data_to_string(cell).to_lowercase();
                    let ttrim = txt.trim().to_string();
                    if code_idx.is_none() && (ttrim == "codigo" || ttrim == "código") {
                        code_idx = Some(ci);
                    }
                    if name_idx.is_none() && (txt.contains("nombre") || txt.contains("asignatura") || txt.contains("descripcion")) {
                        name_idx = Some(ci);
                    }
                }
                // If we didn't find a clean "codigo" header, fall back to looser detection
                if code_idx.is_none() {
                    for (ci, cell) in row.iter().enumerate() {
                        let txt = data_to_string(cell).to_lowercase();
                        if txt.contains("codigo") || txt.contains("código") || txt.contains("cod") || txt.contains("seccion") || txt.contains("sección") {
                            code_idx = Some(ci);
                            break;
                        }
                    }
                }
                break;
            }
        }

        for (row_idx, row) in range.rows().enumerate() {
            if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }

            // If header detected, prefer using the identified columns
            if let Some(h) = header_row_idx {
                if row_idx == h { continue; }
                let code = code_idx.and_then(|i| row.get(i)).map(|c| data_to_string(c).trim().to_string()).unwrap_or_default();
                let name = name_idx.and_then(|i| row.get(i)).map(|c| data_to_string(c).trim().to_string()).unwrap_or_default();
                if code.is_empty() { continue; }
                let lowc = code.to_lowercase();
                if lowc.contains("sección") || lowc.contains("num") || lowc.contains("tipo") || lowc.contains("codigo plan") || lowc == "final" { continue; }
                map.insert(code, name);
                continue;
            }

            // Fallback heuristic when no header: take first two non-empty cells but validate
            let mut code: Option<String> = None;
            let mut name: Option<String> = None;
            for cell in row.iter().take(12) {
                let s = data_to_string(cell).trim().to_string();
                if s.is_empty() { continue; }
                let slow = s.to_lowercase();
                if slow.starts_with("sección") || slow.starts_with("num") || slow.starts_with("tipo") || slow.starts_with("codigo plan") || slow == "final" {
                    code = None;
                    name = None;
                    break;
                }
                if code.is_none() {
                    code = Some(s);
                } else if name.is_none() {
                    name = Some(s);
                    break;
                }
            }
            if let Some(c) = code {
                let is_code_like = c.chars().any(|ch| ch.is_ascii_digit()) || c.contains('_') || c.contains('-');
                if !is_code_like { continue; }
                let n = name.unwrap_or_else(|| "".to_string());
                map.insert(c.trim().to_string(), n.trim().to_string());
            }
        }
    }

    Ok(map)
}

#[test]
fn test_inconsistencias_oa_vs_malla() {
    eprintln!("\n🔎 TEST: Inconsistencias OA vs MC2020.xlsx");

    let (malla_path, _oferta, _porcent) = resolve_datafile_paths("MC2020.xlsx").expect("No se pudo resolver MC2020.xlsx");

    // Buscar posibles ficheros OA disponibles en src/datafiles como fallback
    let oa_candidates = vec![
        "MC2020_mapped_to_OA_majority.xlsx",
        "MC2020_mapped_to_OA.xlsx",
        "OA2024.xlsx",
        "OA_TEST.xlsx",
    ];

    let mut oa_path_opt: Option<PathBuf> = None;
    for cand in oa_candidates.iter() {
        if let Ok((p, _, _)) = resolve_datafile_paths(cand) {
            let pb = PathBuf::from(&p);
            if pb.exists() {
                oa_path_opt = Some(pb);
                break;
            }
        }
    }

    let malla_path = PathBuf::from(malla_path);
    assert!(malla_path.exists(), "Archivo MC2020.xlsx no existe: {:?}", malla_path);

    let oa_path = if let Some(p) = oa_path_opt {
        p
    } else {
        // Si no existe ningún OA candidate, fallar con mensaje claro
        panic!("No se encontró ningún archivo OA candidato en src/datafiles. Buscados: MC2020_mapped_to_OA_majority.xlsx, OA2024.xlsx, OA_TEST.xlsx");
    };

    let malla = read_courses_from_xlsx(&malla_path).expect("Lectura MC2020 falló");
    let oa = read_courses_from_xlsx(&oa_path).expect("Lectura OA falló");

    let mut missing_in_malla: Vec<(String, String)> = Vec::new();
    let mut missing_in_oa: Vec<(String, String)> = Vec::new();
    let mut name_mismatches: Vec<(String, String, String)> = Vec::new();

    for (code, name_oa) in oa.iter() {
        match malla.get(code) {
            None => missing_in_malla.push((code.clone(), name_oa.clone())),
            Some(name_m) => {
                let nm = name_m.trim().to_lowercase();
                let no = name_oa.trim().to_lowercase();
                if nm != no {
                    name_mismatches.push((code.clone(), name_m.clone(), name_oa.clone()));
                }
            }
        }
    }

    for (code, name_m) in malla.iter() {
        if !oa.contains_key(code) {
            missing_in_oa.push((code.clone(), name_m.clone()));
        }
    }

    if missing_in_malla.is_empty() && missing_in_oa.is_empty() && name_mismatches.is_empty() {
        eprintln!("✅ No se encontraron inconsistencias entre OA y MC2020.xlsx");
        return;
    }

    eprintln!("\n❌ Se encontraron inconsistencias entre OA y MC2020.xlsx:\n");

    if !missing_in_malla.is_empty() {
        eprintln!("👉 Cursos presentes en OA pero NO en MC2020.xlsx (posible no_en_malla):");
        for (code, name) in missing_in_malla.iter() {
            eprintln!("   - {}: {}", code, name);
        }
        eprintln!();
    }

    if !missing_in_oa.is_empty() {
        eprintln!("👉 Cursos presentes en MC2020.xlsx pero NO en OA (posible falta de mapeo):");
        for (code, name) in missing_in_oa.iter() {
            eprintln!("   - {}: {}", code, name);
        }
        eprintln!();
    }

    if !name_mismatches.is_empty() {
        eprintln!("👉 Cursos con nombre distinto para mismo código:");
        for (code, name_m, name_oa) in name_mismatches.iter() {
            eprintln!("   - {}: MC2020='{}'  <--->  OA='{}'", code, name_m, name_oa);
        }
        eprintln!();
    }

    panic!("Inconsistencias detectadas entre OA y MC2020.xlsx. Revisa el reporte anterior.");
}
