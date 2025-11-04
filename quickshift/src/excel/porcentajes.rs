use std::collections::HashMap;
use calamine::{open_workbook_auto, Data, Reader};
use crate::excel::io::{data_to_string, read_sheet_via_zip};
use crate::excel::normalize_name;

/// Leer porcentajes/aprobados. Devuelve un mapa codigo -> (A, n) donde
/// A = porcentaje (o estimado), n = total (o 100 si no hay total)
pub fn leer_porcentajes_aprobados(path: &str) -> Result<HashMap<String, (f64, f64)>, Box<dyn std::error::Error>> {
    let mut res: HashMap<String, (f64, f64)> = HashMap::new();

    // Resolver ruta hacia el directorio protegido `DATAFILES_DIR` si el path directo no existe
    let resolved = if std::path::Path::new(path).exists() {
        path.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, path);
        if std::path::Path::new(&candidate).exists() { candidate } else { path.to_string() }
    };

    // Intentar con calamine primero
    if let Ok(mut workbook) = open_workbook_auto(&resolved) {
        let sheet_names = workbook.sheet_names().to_owned();
        if !sheet_names.is_empty() {
            let primera = &sheet_names[0];
            if let Ok(range) = workbook.worksheet_range(primera) {
                let mut rows_iter = range.rows();
                if let Some(header_row) = rows_iter.next() {
                    let headers: Vec<String> = header_row.iter().map(|c| data_to_string(c)).map(|s| s.to_lowercase()).collect();
                    let mut idx_codigo: usize = 0;
                    let mut idx_aprobados: Option<usize> = None;
                    let mut idx_total: Option<usize> = None;
                    let mut idx_porcentaje: Option<usize> = None;
                    for (i, h) in headers.iter().enumerate() {
                        if h.contains("codigo") || h == "ramo" || h == "asignatura" { idx_codigo = i; }
                        if h.contains("aprob") { idx_aprobados = Some(i); }
                        if h.contains("total") { idx_total = Some(i); }
                        if h.contains("porcentaje") || h.contains('%') { idx_porcentaje = Some(i); }
                    }

                for row in rows_iter {
                let codigo = data_to_string(row.get(idx_codigo).unwrap_or(&Data::Empty)).trim().to_string();
                        if codigo.is_empty() { continue; }

                        if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                            let a = data_to_string(row.get(ai).unwrap_or(&Data::Empty)).replace(',', ".");
                            let n = data_to_string(row.get(ni).unwrap_or(&Data::Empty)).replace(',', ".");
                            if let (Ok(av), Ok(nv)) = (a.parse::<f64>(), n.parse::<f64>()) {
                                res.insert(codigo.clone(), (av, nv));
                                continue;
                            }
                        }

                        if let Some(pi) = idx_porcentaje {
                            let p = data_to_string(row.get(pi).unwrap_or(&Data::Empty)).replace('%', "").replace(',', ".");
                            if let Ok(pv) = p.parse::<f64>() { res.insert(codigo.clone(), (pv, 100.0)); continue; }
                        }
                    }
                }
                return Ok(res);
            }
        }
    }

    // fallback: intentar leer con helper (devuelve Vec<Vec<String>>)
    match read_sheet_via_zip(path, "") {
        Ok(rows) => {
            if rows.is_empty() { return Ok(res); }
            let headers_row = &rows[0];
            let headers: Vec<String> = headers_row.iter().map(|h| h.trim().to_lowercase()).collect();
            let mut idx_codigo: usize = 0;
            let mut idx_aprobados: Option<usize> = None;
            let mut idx_total: Option<usize> = None;
            let mut idx_porcentaje: Option<usize> = None;
            for (i, h) in headers.iter().enumerate() {
                if h.contains("codigo") || h == "ramo" || h == "asignatura" { idx_codigo = i; }
                if h.contains("aprob") { idx_aprobados = Some(i); }
                if h.contains("total") { idx_total = Some(i); }
                if h.contains("porcentaje") || h.contains('%') { idx_porcentaje = Some(i); }
            }

            for (i, row) in rows.iter().enumerate() {
                if i == 0 { continue; }
                let codigo = row.get(idx_codigo).cloned().unwrap_or_default().trim().to_string();
                if codigo.is_empty() { continue; }

                if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                    let a = row.get(ai).cloned().unwrap_or_default().replace(',', ".");
                    let n = row.get(ni).cloned().unwrap_or_default().replace(',', ".");
                    if let (Ok(av), Ok(nv)) = (a.parse::<f64>(), n.parse::<f64>()) {
                        res.insert(codigo.clone(), (av, nv));
                        continue;
                    }
                }
                if let Some(pi) = idx_porcentaje {
                    let p = row.get(pi).cloned().unwrap_or_default().replace('%', "").replace(',', ".");
                    if let Ok(pv) = p.parse::<f64>() { res.insert(codigo.clone(), (pv, 100.0)); continue; }
                }

                // fallback segunda columna
                let second = row.get(1).cloned().unwrap_or_default();
                let s2 = second.replace('%', "").replace(',', ".");
                if let Ok(pv) = s2.parse::<f64>() { res.insert(codigo.clone(), (pv, 100.0)); }
            }
            return Ok(res);
        }
        Err(e) => return Err(format!("No se pudo leer porcentajes: {}", e).into()),
    }
}

/// Variante que además intenta extraer el nombre/denominación del ramo y si es electivo
/// para construir un índice nombre_normalizado -> (codigo, porcentaje, total, es_electivo)
/// Este índice se puede usar como fallback para emparejar PA -> malla por nombre.
pub fn leer_porcentajes_aprobados_con_nombres(path: &str) -> Result<(HashMap<String, (f64, f64)>, std::collections::HashMap<String, (String, f64, f64, bool)>), Box<dyn std::error::Error>> {
    let mut res: HashMap<String, (f64, f64)> = HashMap::new();
    let mut name_index: std::collections::HashMap<String, (String, f64, f64, bool)> = std::collections::HashMap::new();

    let resolved = if std::path::Path::new(path).exists() {
        path.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, path);
        if std::path::Path::new(&candidate).exists() { candidate } else { path.to_string() }
    };

    if let Ok(mut workbook) = open_workbook_auto(&resolved) {
        let sheet_names = workbook.sheet_names().to_owned();
        if !sheet_names.is_empty() {
            let primera = &sheet_names[0];
            if let Ok(range) = workbook.worksheet_range(primera) {
                // Collect rows (we will search for a header within the first N rows)
                let rows: Vec<Vec<Data>> = range.rows().map(|r| r.to_vec()).collect();
                // Buscar fila de cabecera en las primeras 8 filas (o menos si el sheet es corto)
                let search_limit = std::cmp::min(8, rows.len());
                let mut header_idx: Option<usize> = None;
                for i in 0..search_limit {
                    let headers: Vec<String> = rows[i].iter().map(|c| data_to_string(c).to_lowercase()).collect();
                    // considerar fila header si contiene 'codigo' o 'ramo' o 'asignatura'
                    if headers.iter().any(|h| h.contains("codigo") || h.contains("ramo") || h.contains("asignatura")) {
                        header_idx = Some(i);
                        break;
                    }
                }

                if let Some(hidx) = header_idx {
                    let headers: Vec<String> = rows[hidx].iter().map(|c| data_to_string(c).to_lowercase()).collect();
                    let mut idx_codigo: usize = 0;
                    let mut idx_aprobados: Option<usize> = None;
                    let mut idx_total: Option<usize> = None;
                    let mut idx_porcentaje: Option<usize> = None;
                    let mut idx_nombre: Option<usize> = None;
                    let mut idx_electivo: Option<usize> = None;
                    for (i, h) in headers.iter().enumerate() {
                        if h.contains("codigo") || h == "ramo" || h == "asignatura" { idx_codigo = i; }
                        if h.contains("aprob") { idx_aprobados = Some(i); }
                        if h.contains("total") { idx_total = Some(i); }
                        if h.contains("porcentaje") || h.contains('%') { idx_porcentaje = Some(i); }
                        if h.contains("denomin") || h.contains("denominación") || h.contains("denominacion") || h.contains("asignatura") { idx_nombre = Some(i); }
                        if h.contains("electivo") { idx_electivo = Some(i); }
                    }

                    for row in rows.iter().skip(hidx+1) {
                        let codigo = data_to_string(row.get(idx_codigo).unwrap_or(&Data::Empty)).trim().to_string();
                        if codigo.is_empty() { continue; }

                        let mut pct: Option<f64> = None;
                        let mut tot: f64 = 100.0;

                        if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                            let a = data_to_string(row.get(ai).unwrap_or(&Data::Empty)).replace(',', ".");
                            let n = data_to_string(row.get(ni).unwrap_or(&Data::Empty)).replace(',', ".");
                            if let (Ok(av), Ok(nv)) = (a.parse::<f64>(), n.parse::<f64>()) {
                                pct = Some(av);
                                tot = nv;
                            }
                        }

                        if pct.is_none() {
                            if let Some(pi) = idx_porcentaje {
                                let p = data_to_string(row.get(pi).unwrap_or(&Data::Empty)).replace('%', "").replace(',', ".");
                                if let Ok(pv) = p.parse::<f64>() { pct = Some(pv); tot = 100.0; }
                            }
                        }

                        // Extraer si es electivo
                        let es_electivo = if let Some(ei) = idx_electivo {
                            let ev = data_to_string(row.get(ei).unwrap_or(&Data::Empty)).to_lowercase();
                            ev == "true" || ev == "1" || ev == "sí" || ev == "si"
                        } else {
                            false
                        };

                        if let Some(pctv) = pct {
                            res.insert(codigo.clone(), (pctv, tot));
                            if let Some(ni) = idx_nombre {
                                let nombre = data_to_string(row.get(ni).unwrap_or(&Data::Empty)).trim().to_string();
                                if !nombre.is_empty() {
                                    let key = normalize_name(&nombre);
                                    name_index.insert(key, (codigo.clone(), pctv, tot, es_electivo));
                                }
                            }
                        }
                    }
                }
                return Ok((res, name_index));
            }
        }
    }

    match read_sheet_via_zip(path, "") {
        Ok(rows) => {
            if rows.is_empty() { return Ok((res, name_index)); }
            let headers_row = &rows[0];
            let headers: Vec<String> = headers_row.iter().map(|h| h.trim().to_lowercase()).collect();
            let mut idx_codigo: usize = 0;
            let mut idx_aprobados: Option<usize> = None;
            let mut idx_total: Option<usize> = None;
            let mut idx_porcentaje: Option<usize> = None;
            let mut idx_nombre: Option<usize> = None;
            let mut idx_electivo: Option<usize> = None;
            for (i, h) in headers.iter().enumerate() {
                if h.contains("codigo") || h == "ramo" || h == "asignatura" { idx_codigo = i; }
                if h.contains("aprob") { idx_aprobados = Some(i); }
                if h.contains("total") { idx_total = Some(i); }
                if h.contains("porcentaje") || h.contains('%') { idx_porcentaje = Some(i); }
                if h.contains("denomin") || h.contains("denominación") || h.contains("denominacion") || h.contains("asignatura") { idx_nombre = Some(i); }
                if h.contains("electivo") { idx_electivo = Some(i); }
            }

            for (i, row) in rows.iter().enumerate() {
                if i == 0 { continue; }
                let codigo = row.get(idx_codigo).cloned().unwrap_or_default().trim().to_string();
                if codigo.is_empty() { continue; }

                let mut pct: Option<f64> = None;
                let mut tot: f64 = 100.0;

                if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                    let a = row.get(ai).cloned().unwrap_or_default().replace(',', ".");
                    let n = row.get(ni).cloned().unwrap_or_default().replace(',', ".");
                    if let (Ok(av), Ok(nv)) = (a.parse::<f64>(), n.parse::<f64>()) {
                        pct = Some(av);
                        tot = nv;
                    }
                }

                if pct.is_none() {
                    if let Some(pi) = idx_porcentaje {
                        let p = row.get(pi).cloned().unwrap_or_default().replace('%', "").replace(',', ".");
                        if let Ok(pv) = p.parse::<f64>() { pct = Some(pv); tot = 100.0; }
                    }
                }

                // Extraer si es electivo
                let es_electivo = if let Some(ei) = idx_electivo {
                    let ev = row.get(ei).cloned().unwrap_or_default().to_lowercase();
                    ev == "true" || ev == "1" || ev == "sí" || ev == "si"
                } else {
                    false
                };

                if let Some(pctv) = pct {
                    res.insert(codigo.clone(), (pctv, tot));
                    if let Some(ni) = idx_nombre {
                        let nombre = row.get(ni).cloned().unwrap_or_default().trim().to_string();
                        if !nombre.is_empty() {
                            let key = normalize_name(&nombre);
                            name_index.insert(key, (codigo.clone(), pctv, tot, es_electivo));
                        }
                    }
                }
            }
            return Ok((res, name_index));
        }
        Err(e) => return Err(format!("No se pudo leer porcentajes: {}", e).into()),
    }
}

/// Enriquecer porcent_names vacío usando nombres de Malla.
/// Si porcent_names está vacío (porque PA no tiene columna "nombre"),
/// intentamos matchear PA codes a Malla courses por nombre normalizado.
/// 
/// Estrategia:
/// 1. Primero: tratar de encontrar coincidencias por nombre normalizado (si PA code 
///    coincide con algún nombre de Malla normalizado)
/// 2. Fallback: asignación ordenada 1:1 para los que no matchearon
pub fn enrich_porcent_names_from_malla(
    porcent_names: &mut std::collections::HashMap<String, (String, f64, f64, bool)>,
    porcent: &HashMap<String, (f64, f64)>,
    malla_map: &std::collections::HashMap<String, crate::models::RamoDisponible>,
) {
    if porcent_names.is_empty() && !porcent.is_empty() {
        use crate::excel::io::normalize_name;
        
        // Construir índice de nombres de Malla normalizados
        let mut malla_by_norm: std::collections::HashMap<String, (String, &crate::models::RamoDisponible)> 
            = std::collections::HashMap::new();
        for (mcode, ramo) in malla_map.iter() {
            let rname_norm = normalize_name(&ramo.nombre);
            malla_by_norm.insert(rname_norm, (mcode.clone(), ramo));
        }
        
        eprintln!("[ENRICH] Building porcent_names from PA data...");
        eprintln!("[ENRICH] Total PA codes: {}, Total Malla courses: {}", porcent.len(), malla_map.len());
        
        let mut matched = 0;
        let mut unmatched_pa: Vec<(String, f64, f64)> = Vec::new();
        let mut unmatched_malla: Vec<(String, String)> = Vec::new();
        
        // PASO 1: Intentar matchear PA codes a nombres de Malla normalizados
        for (pa_code, (pct, tot)) in porcent.iter() {
            let pa_norm = normalize_name(pa_code);
            
            if let Some((mcode, _ramo)) = malla_by_norm.get(&pa_norm) {
                // ¡Encontramos match por nombre normalizado!
                porcent_names.insert(pa_norm.clone(), (pa_code.clone(), *pct, *tot, false));
                eprintln!("[ENRICH] MATCHED by name: PA code '{}' -> Malla '{}' (pct={}%, tot={})", 
                    pa_code, mcode, pct, tot);
                matched += 1;
            } else {
                // No matcheó por nombre, guardar para asignación ordenada
                unmatched_pa.push((pa_code.clone(), *pct, *tot));
            }
        }
        
        // PASO 2: Recolectar ramos de Malla que no fueron matcheados
        let matched_rnames: std::collections::HashSet<String> = porcent_names.keys().cloned().collect();
        for (rname_norm, (mcode, _ramo)) in &malla_by_norm {
            if !matched_rnames.contains(rname_norm) {
                unmatched_malla.push((rname_norm.clone(), mcode.clone()));
            }
        }
        
        // PASO 3: Asignación 1:1 ordenada para los no matcheados
        unmatched_pa.sort_by(|a, b| a.0.cmp(&b.0));
        unmatched_malla.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (i, (pa_code, pct, tot)) in unmatched_pa.iter().enumerate() {
            if i < unmatched_malla.len() {
                let (rname_norm, mcode) = &unmatched_malla[i];
                porcent_names.insert(rname_norm.clone(), (pa_code.clone(), *pct, *tot, false));
                eprintln!("[ENRICH] FALLBACK 1:1: PA code '{}' -> Malla '{}' (pct={}%, tot={})", 
                    pa_code, mcode, pct, tot);
            }
        }
        
        eprintln!("[ENRICH] ✅ Complete! Matched: {}, Unmatched PA: {}, Unmatched Malla: {}, Final size: {}", 
            matched, unmatched_pa.len(), unmatched_malla.len(), porcent_names.len());
    }
}

