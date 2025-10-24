use std::collections::HashMap;
use calamine::{open_workbook_auto, Data, Reader};
use crate::models::RamoDisponible;
use crate::excel::io::data_to_string;

/// Lee un archivo de malla (espera filas: codigo, nombre, correlativo, holgura, critico, ...)
pub fn leer_malla_excel(nombre_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    let mut workbook = open_workbook_auto(nombre_archivo)?;
    let mut ramos_disponibles = HashMap::new();

    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    let primera_hoja = &sheet_names[0];
    let range = workbook.worksheet_range(primera_hoja)?;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }

        let codigo = data_to_string(row.get(0).unwrap_or(&Data::Empty));
        let nombre = data_to_string(row.get(1).unwrap_or(&Data::Empty));

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
