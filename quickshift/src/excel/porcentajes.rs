use std::collections::HashMap;
use calamine::{open_workbook_auto, Data, Reader};
use crate::excel::io::{data_to_string, read_sheet_via_zip};

/// Leer porcentajes/aprobados. Devuelve un mapa codigo -> (A, n) donde
/// A = porcentaje (o estimado), n = total (o 100 si no hay total)
pub fn leer_porcentajes_aprobados(path: &str) -> Result<HashMap<String, (f64, f64)>, Box<dyn std::error::Error>> {
    let mut res: HashMap<String, (f64, f64)> = HashMap::new();

    // Intentar con calamine primero
    if let Ok(mut workbook) = open_workbook_auto(path) {
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
