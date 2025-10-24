use calamine::{open_workbook_auto, Data, Reader};
use crate::models::Seccion;
use crate::excel::io::{data_to_string, read_sheet_via_zip};

/// Lee la oferta académica y devuelve una lista de `Seccion`.
pub fn leer_oferta_academica_excel(nombre_archivo: &str) -> Result<Vec<Seccion>, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(nombre_archivo)?;
    let mut secciones = Vec::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    // Candidate sheets: prefer names comunes, luego todas
    let mut candidates: Vec<String> = vec!["Mi Malla".to_string(), "MiMalla".to_string(), "Mi malla".to_string()];
    for s in sheet_names.iter() { if !candidates.contains(s) { candidates.push(s.clone()); } }

    let mut range_opt = None;
    let mut used_sheet: Option<String> = None;
    let mut rows_vec_opt: Option<Vec<Vec<String>>> = None;

    for cand in candidates.iter() {
        match workbook.worksheet_range(cand) {
            Ok(rng) => { range_opt = Some(rng); used_sheet = Some(cand.clone()); break; }
            Err(_) => {
                // intentar fallback
                if let Ok(rows) = read_sheet_via_zip(nombre_archivo, cand) {
                    if !rows.is_empty() { rows_vec_opt = Some(rows); used_sheet = Some(cand.clone()); break; }
                }
            }
        }
    }

    if let Some(range) = range_opt {
        let rows: Vec<_> = range.rows().collect();
        if rows.len() < 2 { return Err("El archivo debe tener al menos 2 filas (header + datos)".into()); }

        for (row_idx, row) in rows.iter().enumerate() {
            if row_idx == 0 { continue; }
            if row.is_empty() { continue; }

            let codigo = data_to_string(row.get(0).unwrap_or(&Data::Empty));
            if codigo.trim().is_empty() { continue; }
            let nombre = data_to_string(row.get(1).unwrap_or(&Data::Empty));
            let seccion = data_to_string(row.get(2).unwrap_or(&Data::Empty));
            let horario_str = data_to_string(row.get(3).unwrap_or(&Data::Empty));
            let profesor = data_to_string(row.get(4).unwrap_or(&Data::Empty));
            let codigo_box = data_to_string(row.get(5).unwrap_or(&Data::Empty));

            let codigo_box = if codigo_box.is_empty() {
                if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() }
            } else { codigo_box };

            let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else {
                horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            };

            secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
        }

        return Ok(secciones);
    }

    if let Some(rows_vec) = rows_vec_opt {
        if rows_vec.len() < 2 { return Err("El archivo debe tener al menos 2 filas (header + datos)".into()); }
        for (row_idx, row) in rows_vec.iter().enumerate() {
            if row_idx == 0 { continue; }
            if row.iter().all(|c| c.trim().is_empty()) { continue; }

            let codigo = row.get(0).cloned().unwrap_or_default();
            if codigo.trim().is_empty() { continue; }
            let nombre = row.get(1).cloned().unwrap_or_else(|| "Sin nombre".to_string());
            let seccion = row.get(2).cloned().unwrap_or_else(|| "1".to_string());
            let horario_str = row.get(3).cloned().unwrap_or_default();
            let profesor = row.get(4).cloned().unwrap_or_else(|| "Sin asignar".to_string());
            let codigo_box = row.get(5).cloned().unwrap_or_else(|| if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() });
            let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else { horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() };

            secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
        }
        return Ok(secciones);
    }

    Err(format!("No se pudo leer ninguna hoja del archivo '{}'.", nombre_archivo).into())
}
