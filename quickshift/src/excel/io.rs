use calamine::{open_workbook_auto, Data};
use std::path::Path;

/// Convierte un `Data` de calamine a String (versión genérica para celdas)
pub fn cell_to_string(c: &Data) -> String {
    match c {
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if (f.floor() - f).abs() < std::f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Data::Int(i) => format!("{}", i),
        Data::Bool(b) => format!("{}", b),
        Data::Empty => String::new(),
        Data::Error(_) => String::new(),
        Data::DateTime(s) => s.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
    }
}

/// Convierte un `Data` de calamine a String (útil cuando se usa Xlsx::worksheet_range)
pub fn data_to_string(d: &Data) -> String {
    match d {
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        Data::Empty => String::new(),
        Data::Error(_) => String::new(),
        Data::DateTime(s) => s.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
    }
}

/// Normaliza encabezados eliminando espacios y pasando a minúsculas.
pub fn normalize_header(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect()
}

/// Convierte letras de columna (ej: "AB") a índice 1-based (A=1)
pub fn column_letters_to_index(s: &str) -> usize {
    let mut acc = 0usize;
    for ch in s.chars() {
        if ch.is_ascii_alphabetic() {
            acc = acc * 26 + ((ch.to_ascii_uppercase() as u8 - b'A') as usize + 1);
        }
    }
    acc
}

/// Intenta leer una hoja del archivo Excel y devolverla como Vec<Vec<String>>.
/// Implementación basada en `calamine::open_workbook_auto` para simplicidad (sirve como fallback)
pub fn read_sheet_via_zip<P: AsRef<Path>>(path: P, sheet_name: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    use calamine::Reader;
    let mut workbook = open_workbook_auto(path)?;

    // Preferir la hoja con el nombre solicitado; si no existe, tomar la primera
    let names = workbook.sheet_names().to_owned();
    let sheet_to_use = if sheet_name.is_empty() {
        names.get(0).cloned().unwrap_or_default()
    } else {
        names.iter().find(|s| *s == sheet_name).cloned().unwrap_or_else(|| names.get(0).cloned().unwrap_or_default())
    };

    if sheet_to_use.is_empty() {
        return Ok(Vec::new());
    }

    match workbook.worksheet_range(&sheet_to_use) {
        Ok(range) => {
            let mut rows: Vec<Vec<String>> = Vec::new();
            for r in range.rows() {
                let mut row_vec: Vec<String> = Vec::new();
                for cell in r.iter() {
                    row_vec.push(cell_to_string(cell));
                }
                rows.push(row_vec);
            }
            Ok(rows)
        }
        Err(_) => Ok(Vec::new()),
    }
}
