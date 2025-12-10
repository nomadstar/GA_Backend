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
        match workbook.worksheet_range(&sheet_name) {
            Ok(range) => {
                for row in range.rows() {
                    if row.iter().all(|c| matches!(c, Data::Empty)) { continue; }
                    let mut code: Option<String> = None;
                    let mut name: Option<String> = None;
                    for cell in row.iter().take(8) {
                        let s = data_to_string(cell).trim().to_string();
                        if s.is_empty() { continue; }
                        if code.is_none() {
                            code = Some(s);
                        } else if name.is_none() {
                            name = Some(s);
                            break;
                        }
                    }
                    if let Some(c) = code {
                        let n = name.unwrap_or_default();
                        map.insert(c.trim().to_string(), n.trim().to_string());
                    }
                }
            }
            Err(_) => continue,
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
