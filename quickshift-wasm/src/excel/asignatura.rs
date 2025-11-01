use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;
use crate::excel::io::{cell_to_string, normalize_header};

/// Busca en el archivo Excel la fila cuyo "Nombre Asignado" coincide con `nombre_asignado`
/// y retorna el valor de la columna "Asignatura" si se encuentra.
pub fn asignatura_from_nombre<P: AsRef<Path>>(path: P, nombre_asignado: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(path)?;
    let target_norm = nombre_asignado.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect::<String>();

    for sheet_name in workbook.sheet_names().to_owned() {
        match workbook.worksheet_range(&sheet_name) {
            Ok(range) => {
                let mut rows = range.rows();
                let header = match rows.next() {
                    Some(h) => h,
                    None => continue,
                };

                let mut idx_nombre: Option<usize> = None;
                let mut idx_asignatura: Option<usize> = None;

                for (i, cell) in header.iter().enumerate() {
                    let text = cell_to_string(cell);
                    let norm = normalize_header(&text);
                    if norm == normalize_header("Nombre Asignado") { idx_nombre = Some(i); }
                    if norm == normalize_header("Asignatura") { idx_asignatura = Some(i); }
                }

                let (i_nombre, i_asig) = match (idx_nombre, idx_asignatura) {
                    (Some(a), Some(b)) => (a, b),
                    _ => continue,
                };

                for row in rows {
                    let nombre_cell = row.get(i_nombre).unwrap_or(&Data::Empty);
                    let asign_cell = row.get(i_asig).unwrap_or(&Data::Empty);
                    let nombre_val = cell_to_string(nombre_cell);
                    let nombre_val_norm = nombre_val.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect::<String>();

                    if nombre_val_norm == target_norm {
                        let asign_val = cell_to_string(asign_cell);
                        return Ok(Some(asign_val));
                    }
                }
            }
            Err(_) => continue,
        }
    }

    Ok(None)
}
