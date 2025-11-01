use std::path::Path;
use calamine::Data;
// buffer-based parsing is optional behind the "excel" feature
#[cfg(feature = "excel")]
use std::io::Cursor;
#[cfg(feature = "excel")]
use calamine::{open_workbook_auto, Xlsx, Reader};

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

/// Normaliza un nombre human-readable: minusculas, elimina acentos, convierte
/// puntuación a espacios y colapsa espacios múltiples.
pub fn normalize_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    // mapa simple de acentos comunes en español/latam
    for ch in s.chars() {
        let c = match ch {
            'Á' | 'À' | 'Ä' | 'Â' | 'Ã' | 'á' | 'à' | 'ä' | 'â' | 'ã' => 'a',
            'É' | 'È' | 'Ë' | 'Ê' | 'é' | 'è' | 'ë' | 'ê' => 'e',
            'Í' | 'Ì' | 'Ï' | 'Î' | 'í' | 'ì' | 'ï' | 'î' => 'i',
            'Ó' | 'Ò' | 'Ö' | 'Ô' | 'Õ' | 'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
            'Ú' | 'Ù' | 'Ü' | 'Û' | 'ú' | 'ù' | 'ü' | 'û' => 'u',
            'Ñ' | 'ñ' => 'n',
            'Ç' | 'ç' => 'c',
            other => other,
        };

        // permitir letras, dígitos y espacios; reemplazar cualquier otra cosa por espacio
        if c.is_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c.is_whitespace() {
            out.push(' ');
        } else {
            // punctuation -> space
            out.push(' ');
        }
    }

    // colapsar espacios múltiples
    let mut res = String::with_capacity(out.len());
    let mut prev_space = false;
    for ch in out.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                res.push(' ');
                prev_space = true;
            }
        } else {
            res.push(ch);
            prev_space = false;
        }
    }

    res.trim().to_string()
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

// Nueva API: leer desde un buffer en memoria (bytes de un .xlsx)
// Disponible solo si la feature "excel" está activada.
#[cfg(feature = "excel")]
pub fn read_sheet_from_buffer(bytes: &[u8], sheet_name: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let cur = Cursor::new(bytes);
    let mut workbook: Xlsx<Cursor<&[u8]>> = Xlsx::new(cur)?;

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
        Err(e) => Err(Box::new(e)),
    }
}

// Si la feature "excel" no está activa, ofrecemos un stub que devuelve error claro.
#[cfg(not(feature = "excel"))]
pub fn read_sheet_from_buffer(_bytes: &[u8], _sheet_name: &str) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    Err("feature \"excel\" not enabled: read_sheet_from_buffer unavailable".into())
}
