use std::collections::HashMap;
use calamine::{open_workbook_auto, Data, Reader};
use crate::models::RamoDisponible;
use crate::excel::io::data_to_string;
use std::path::Path;

/// Lee un archivo de malla (espera filas: codigo, nombre, correlativo, holgura, critico, ...)
/// Leer malla desde un archivo Excel, permitiendo opcionalmente elegir la hoja
/// por nombre. Si `sheet` es None se usa la primera hoja del workbook.
pub fn leer_malla_excel_with_sheet(nombre_archivo: &str, sheet: Option<&str>) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    // Resolver ruta: si el path directo no existe, intentar buscar en el directorio protegido `DATAFILES_DIR`
    let resolved = if std::path::Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, nombre_archivo);
        if std::path::Path::new(&candidate).exists() { candidate } else { nombre_archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(resolved)?;
    let mut ramos_disponibles = HashMap::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    // Elegir hoja: prioridad -> sheet (si provisto y existe), else primera hoja
    let hoja_seleccionada = if let Some(s) = sheet {
        if sheet_names.iter().any(|n| n == s) { s.to_string() } else { sheet_names[0].clone() }
    } else {
        sheet_names[0].clone()
    };

    let range = workbook.worksheet_range(&hoja_seleccionada)?;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }

        // Leer las dos primeras columnas (pueden venir como "ID | Nombre" o
        // como "Nombre | ID" según el archivo). Normalizamos su orden con una
        // función auxiliar que encapsula la heurística de detección.
        let col0 = data_to_string(row.get(0).unwrap_or(&Data::Empty));
        let col1 = data_to_string(row.get(1).unwrap_or(&Data::Empty));
        let (codigo, nombre) = normalize_codigo_nombre(&col0, &col1);

        let correlativo = data_to_string(row.get(2).unwrap_or(&Data::Empty)).parse::<i32>().unwrap_or(0);
        let holgura = data_to_string(row.get(3).unwrap_or(&Data::Empty)).parse::<i32>().unwrap_or(0);

        let critico = {
            let v = data_to_string(row.get(4).unwrap_or(&Data::Empty));
            let vlow = v.to_lowercase();
            if vlow == "true" { true }
            else if let Ok(n) = v.parse::<i32>() { n != 0 }
            else if let Ok(f) = v.parse::<f64>() { f != 0.0 }
            else { false }
        };

        if !codigo.is_empty() {
            ramos_disponibles.insert(codigo.clone(), RamoDisponible {
                nombre,
                codigo: codigo.clone(),
                holgura,
                numb_correlativo: correlativo,
                critico,
                codigo_ref: Some(codigo),
                dificultad: None,
            });
        }
    }

    Ok(ramos_disponibles)
}

/// Normaliza el par (col0, col1) devolviendo (codigo, nombre).
/// Si detecta que la primera columna contiene letras y la segunda contiene
/// dígitos (por ejemplo: "Nombre" | "ID"), invierte el orden para que el
/// resultado sea siempre (ID, Nombre).
fn normalize_codigo_nombre(col0: &str, col1: &str) -> (String, String) {
    let mut codigo = col0.to_string();
    let mut nombre = col1.to_string();
    let first_has_alpha = codigo.chars().any(|c| c.is_alphabetic());
    let second_has_digit = nombre.chars().any(|c| c.is_digit(10));
    if first_has_alpha && second_has_digit {
        std::mem::swap(&mut codigo, &mut nombre);
    }
    (codigo, nombre)
}

#[cfg(test)]
mod tests {
    use super::normalize_codigo_nombre;

    #[test]
    fn detect_swap_nombre_id() {
        let nombre = "Álgebra y Geometría";
        let id = "1";
        let (codigo, nombre_out) = normalize_codigo_nombre(nombre, id);
        assert_eq!(codigo, "1");
        assert_eq!(nombre_out, "Álgebra y Geometría");
    }

    #[test]
    fn keep_id_nombre() {
        let id = "7";
        let nombre = "Cálculo II";
        let (codigo, nombre_out) = normalize_codigo_nombre(id, nombre);
        assert_eq!(codigo, "7");
        assert_eq!(nombre_out, "Cálculo II");
    }
}

/// Compat wrapper existente que conserva el nombre original y usa la primera hoja
pub fn leer_malla_excel(nombre_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    leer_malla_excel_with_sheet(nombre_archivo, None)
}

/// Lee hojas adicionales de la malla para extraer prerequisitos.
/// Se espera que cada hoja adicional tenga al menos dos columnas:
/// - columna 0: codigo de la asignatura
/// - columna 1: prerequisitos (puede contener varios códigos separados por ',' o ';')
pub fn leer_prerequisitos(nombre_archivo: &str) -> Result<HashMap<String, Vec<String>>, Box<dyn std::error::Error>> {
    // Resolver ruta: si el path directo no existe, intentar buscar en el directorio protegido `DATAFILES_DIR`
    let resolved = if Path::new(nombre_archivo).exists() {
        nombre_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, nombre_archivo);
        if Path::new(&candidate).exists() { candidate } else { nombre_archivo.to_string() }
    };

    let mut workbook = open_workbook_auto(resolved)?;
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.len() <= 1 {
        // no hay hojas adicionales con prerequisitos
        return Ok(map);
    }

    // Iterar sobre las hojas a partir de la segunda
    for sheet in sheet_names.iter().skip(1) {
        if let Ok(range) = workbook.worksheet_range(sheet) {
            for (row_idx, row) in range.rows().enumerate() {
                if row_idx == 0 { continue; }
                let codigo = data_to_string(row.get(0).unwrap_or(&Data::Empty));
                let raw_pr = data_to_string(row.get(1).unwrap_or(&Data::Empty));
                if codigo.is_empty() || raw_pr.is_empty() { continue; }
                // separar por comas o punto y coma
                let mut list: Vec<String> = raw_pr.split(|c| c==',' || c==';')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !list.is_empty() {
                    map.entry(codigo.clone()).or_insert_with(Vec::new).append(&mut list);
                }
            }
        }
    }

    Ok(map)
}
