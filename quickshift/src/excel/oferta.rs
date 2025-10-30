use calamine::{open_workbook_auto, Data, Reader};
use crate::models::Seccion;
use crate::excel::io::{data_to_string, read_sheet_via_zip};

/// Lee la oferta académica y devuelve una lista de `Seccion`.
pub fn leer_oferta_academica_excel(nombre_archivo: &str) -> Result<Vec<Seccion>, Box<dyn std::error::Error>> {
    // Resolver ruta hacia el directorio protegido `DATAFILES_DIR` si es necesario
    let resolved = if std::path::Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, nombre_archivo);
        if std::path::Path::new(&candidate).exists() { candidate } else { nombre_archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(resolved)?;
    let mut secciones = Vec::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    // We'll iterate all sheets and try to detect a header row by keywords so the
    // parser is robust frente a hojas con filas iniciales de metadatos o cabeceras
    // con nombres no exactamente iguales.
    let header_keywords = ["asignatura", "denominación", "denominacion", "codigo", "código", "id ramo", "id. ramo", "denominación asignatura", "denominación_asignatura", "denominacion asignatura"];

    for sheet in sheet_names.iter() {
        // Try to read via calamine first
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Ok(range) = workbook.worksheet_range(sheet) {
            for r in range.rows() {
                let mut rowv: Vec<String> = Vec::new();
                for c in r.iter() { rowv.push(data_to_string(c)); }
                rows.push(rowv);
            }
        } else {
            // fallback: try zip reader
            if let Ok(rv) = read_sheet_via_zip(nombre_archivo, sheet) {
                rows = rv;
            } else {
                continue; // next sheet
            }
        }

        if rows.len() < 2 { continue; }

        // search for header row among the first 8 rows
        let search_limit = std::cmp::min(8, rows.len());
        let mut header_row_idx: Option<usize> = None;
        for i in 0..search_limit {
            let row = &rows[i];
            let mut matches = 0usize;
            for cell in row.iter() {
                let cl = cell.to_lowercase();
                for kw in header_keywords.iter() {
                    if cl.contains(kw) { matches += 1; break; }
                }
            }
            if matches >= 1 {
                header_row_idx = Some(i);
                break;
            }
        }

        // If we found a header row, try to map columns
        if let Some(hidx) = header_row_idx {
            let header = &rows[hidx];
            // helper to find a column index by matching any of labels
            let find_col = |labels: &[&str]| -> Option<usize> {
                for (ci, cell) in header.iter().enumerate() {
                    let c = cell.to_lowercase();
                    for lab in labels.iter() {
                        if c.contains(lab) { return Some(ci); }
                    }
                }
                None
            };

            let codigo_labels = ["codigo", "código", "id ramo", "id. ramo", "codigo asignatura", "codigo_asignatura", "id.ramo"];
            let nombre_labels = ["denominación asignatura", "denominacion asignatura", "denominación", "denominacion", "asignatura", "denominación asignatura"];
            let seccion_labels = ["sección", "seccion", "sección asignatura", "seccion asignatura"];
            let horario_labels = ["horario", "horarios"];
            let profesor_labels = ["profesor", "docente", "academico"];

            let col_codigo = find_col(&codigo_labels).or_else(|| Some(0)).unwrap();
            let col_nombre = find_col(&nombre_labels).or_else(|| Some(1)).unwrap();
            let col_seccion = find_col(&seccion_labels).unwrap_or(2);
            let col_horario = find_col(&horario_labels).unwrap_or(3);
            let col_profesor = find_col(&profesor_labels).unwrap_or(4);

            // parse rows after header
            for row_idx in (hidx+1)..rows.len() {
                let row = &rows[row_idx];
                // get cell helper
                let cell_at = |r: &Vec<String>, idx: usize| -> String { r.get(idx).cloned().unwrap_or_default() };
                let codigo = cell_at(row, col_codigo).trim().to_string();
                if codigo.is_empty() { continue; }
                let nombre = cell_at(row, col_nombre).trim().to_string();
                if nombre.is_empty() { continue; }
                let seccion = cell_at(row, col_seccion).trim().to_string();
                let horario_str = cell_at(row, col_horario).trim().to_string();
                let profesor = cell_at(row, col_profesor).trim().to_string();
                let mut codigo_box = codigo.clone();
                if codigo_box.is_empty() { codigo_box = codigo.clone(); }
                // split horario
                let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else { horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() };
                secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
            }

            if !secciones.is_empty() { return Ok(secciones); }
        }
    }

    // Fallback: try existing simple parsing from the first sheet available
    // Try each sheet again and parse using positional columns
    for sheet in sheet_names.iter() {
        if let Ok(range) = workbook.worksheet_range(sheet) {
            let rows: Vec<_> = range.rows().collect();
            if rows.len() < 2 { continue; }
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
                let codigo_box = if codigo_box.is_empty() { if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() } } else { codigo_box };
                let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else { horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() };
                secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
            }
            if !secciones.is_empty() { return Ok(secciones); }
        }
        // fallback zip
        if let Ok(rows_vec) = read_sheet_via_zip(nombre_archivo, sheet) {
            if rows_vec.len() < 2 { continue; }
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
            if !secciones.is_empty() { return Ok(secciones); }
        }
    }

    Err(format!("No se pudo leer ninguna hoja del archivo '{}'.", nombre_archivo).into())
}
