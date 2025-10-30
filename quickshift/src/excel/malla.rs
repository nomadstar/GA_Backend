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
        
        // Filtrar filas de encabezado duplicadas ("Código", "ID", etc.)
        let col0_lower = col0.to_lowercase();
        let col1_lower = col1.to_lowercase();
        if (col0_lower == "código" || col0_lower == "id" || col0_lower == "código asignatura" || col0_lower == "asignatura") &&
           (col1_lower == "nombre" || col1_lower == "id" || col1_lower == "código" || col1_lower == "código asignatura" || col1_lower == "asignatura") {
            eprintln!("DEBUG: Saltando fila de encabezado: '{}' || '{}'", col0, col1);
            continue;
        }
        
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

        if !codigo.is_empty() && codigo.to_lowercase() != "código" && codigo.to_lowercase() != "id" {
            // Convertir código a i32 para usar como ID
            let id_num = codigo.parse::<i32>().unwrap_or(0);
            
            // Indexar por nombre normalizado (llave universal única)
            let nombre_norm = crate::excel::normalize_name(&nombre);
            ramos_disponibles.insert(nombre_norm, RamoDisponible {
                id: id_num,
                nombre,
                codigo: codigo.clone(),
                holgura,
                numb_correlativo: correlativo,
                critico,
                codigo_ref: None,  // Se resuelve después si es necesario
                dificultad: None,
                electivo: false,
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

/// Lee Malla2020 y lo enriquece con información de PA2025-1 (porcentajes y códigos)
/// 
/// IMPORTANTE: Manejo especial de ELECTIVOS
/// Los electivos se repiten en Malla2020 (ej: "Electivo Profesional" con múltiples IDs)
/// Por eso indexamos diferente:
/// - NO-ELECTIVOS: clave = nombre_normalizado (universal)
/// - ELECTIVOS: clave = codigo de PA2025-1 (único para cada opción de electivo)
/// 
/// Flujo:
/// 1. Lee PA2025-1 para extraer mapeo: nombre_normalizado → (código, porcentaje, total, es_electivo)
/// 2. Lee Malla2020 (Nombre, ID, Créditos, Requisitos, Semestre, Electivo)
/// 3. Por cada ramo en Malla2020:
///    a. Si es NO-ELECTIVO: normaliza nombre y busca en PA2025-1
///    b. Si es ELECTIVO: busca todos los códigos en PA2025-1 con Electivo=TRUE
///       y selecciona el que tenga MEJOR porcentaje (menor tasa de reprobación)
/// 4. SEGUNDO PASE: Resuelve dependencias por ID
/// 
/// Retorna: HashMap con claves diferenciadas:
/// - NO-ELECTIVOS: nombre_normalizado
/// - ELECTIVOS: codigo de PA2025-1 (ej: "CIT2020", "CBF1001")
pub fn leer_malla_con_porcentajes(malla_archivo: &str, porcentajes_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    use crate::excel::normalize_name;
    use crate::excel::porcentajes::leer_porcentajes_aprobados_con_nombres;
    
    // 1. Leer porcentajes y construir índice por nombre normalizado
    let (_porcent_by_code, porcent_by_name) = leer_porcentajes_aprobados_con_nombres(porcentajes_archivo)?;
    
    // 2. Construir índice invertido: si es_electivo en PA2025-1, indexar también por codigo
    let mut porcent_by_code_electivos: HashMap<String, (String, f64, f64, bool)> = HashMap::new();
    for (norm_name, (codigo, pct, tot, es_electivo)) in porcent_by_name.iter() {
        if *es_electivo {
            porcent_by_code_electivos.insert(codigo.clone(), (codigo.clone(), *pct, *tot, *es_electivo));
        }
    }
    
    // 3. Recopilar todos los electivos disponibles en PA2025-1 (para búsqueda posterior)
    let mut todos_electivos: Vec<(String, f64, f64)> = Vec::new();
    for (codigo, pct, tot, es_electivo) in porcent_by_code_electivos.values() {
        if *es_electivo {
            todos_electivos.push((codigo.clone(), *pct, *tot));
        }
    }
    
    // 4. Leer Malla2020
    let resolved = if Path::new(malla_archivo).exists() {
        malla_archivo.to_string()
    } else {
        let candidate = format!("{}/{}", crate::excel::DATAFILES_DIR, malla_archivo);
        if Path::new(&candidate).exists() { candidate } else { malla_archivo.to_string() }
    };
    
    let mut workbook = open_workbook_auto(resolved)?;
    let mut ramos_disponibles = HashMap::new();
    
    // Usar hoja "Malla2020"
    let range = workbook.worksheet_range("Malla2020")?;
    
    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; }  // Saltar encabezado
        
        // Estructura de Malla2020: Nombre, ID, Créditos, Requisitos, Semestre, Electivo
        let nombre = data_to_string(row.get(0).unwrap_or(&Data::Empty)).trim().to_string();
        let id_str = data_to_string(row.get(1).unwrap_or(&Data::Empty)).trim().to_string();
        let id = id_str.parse::<i32>().unwrap_or(0);
        
        // Leer columna Electivo (column 5)
        let es_electivo_en_malla = {
            let ev = data_to_string(row.get(5).unwrap_or(&Data::Empty)).to_lowercase();
            ev == "true" || ev == "1" || ev == "sí" || ev == "si"
        };
        
        if nombre.is_empty() || id == 0 {
            continue;
        }
        
        // DIFERENCIA CLAVE: usar estrategia diferente para electivos vs no-electivos
        let (clave_hashmap, codigo_final, dificultad, es_electivo_final) = if es_electivo_en_malla {
            // PARA ELECTIVOS: Usar una clave ÚNICA que combine el nombre y el ID
            // Esto permite que cada "Electivo Profesional" (ID 44, 46, 50, 51, 52) sea tratado por separado
            // pero comparten la misma lista de opciones de electivos en PA2025-1
            
            // Buscar en PA2025-1 por Electivo=TRUE y elegir el de MEJOR porcentaje (más fácil)
            let mejor_electivo = todos_electivos.iter()
                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            
            if let Some((cod_elec, pct_elec, _tot_elec)) = mejor_electivo {
                // Clave única: "electivo_profesional_{id}" para evitar colisiones
                let clave_unica = format!("electivo_profesional_{}", id);
                (
                    clave_unica,  // CLAVE = "electivo_profesional_44", "electivo_profesional_46", etc.
                    cod_elec.clone(),  // CÓDIGO = CIT3501, CII2002, etc.
                    Some(*pct_elec),
                    true
                )
            } else {
                // Si no hay electivos en PA2025-1, usar código con ID como fallback
                let clave_unica = format!("electivo_profesional_{}", id);
                (clave_unica, id_str.clone(), None, true)
            }
        } else {
            // PARA NO-ELECTIVOS: usar nombre normalizado como clave
            let nombre_norm = normalize_name(&nombre);
            
            // Buscar en porcentajes por nombre normalizado
            let (codigo, dificultad, es_elec_porcent) = if let Some((codigo_encontrado, porcentaje, _total, es_electivo_en_porcent)) = porcent_by_name.get(&nombre_norm) {
                (codigo_encontrado.clone(), Some(*porcentaje), *es_electivo_en_porcent)
            } else {
                (id_str.clone(), None, false)
            };
            
            (nombre_norm, codigo, dificultad, es_elec_porcent)
        };
        
        eprintln!("DEBUG enrich_malla: '{}' (id={}, electivo={}) → clave='{}', código='{}', dificultad={:?}", 
                  nombre, id, es_electivo_en_malla, clave_hashmap, codigo_final, dificultad);
        
        // Crear RamoDisponible enriquecido (SIN codigo_ref aún, se resuelve en segundo pase)
        let ramo = RamoDisponible {
            id,
            nombre: nombre.clone(),
            codigo: codigo_final.clone(),
            holgura: 0,
            numb_correlativo: id,  // Correlativo es el mismo que ID
            critico: false,
            codigo_ref: None,  // Se resuelve después
            dificultad,
            electivo: es_electivo_final,
        };
        
        // INSERTAR CON CLAVE DIFERENCIADA
        ramos_disponibles.insert(clave_hashmap, ramo);
    }
    
    // SEGUNDO PASE: Resolver dependencias por correlativo
    // Si ramo.numb_correlativo == X, buscar ramo con numb_correlativo == X-1
    // Si existe, establecer codigo_ref al ID del ramo anterior
    let mut updates: Vec<(String, i32)> = Vec::new();
    
    for (clave, ramo) in ramos_disponibles.iter() {
        let correlativo_actual = ramo.numb_correlativo;
        let id_anterior = correlativo_actual - 1;
        
        // Buscar si existe un ramo con numb_correlativo == id_anterior
        for (_, otro_ramo) in ramos_disponibles.iter() {
            if otro_ramo.numb_correlativo == id_anterior {
                // Encontrado: el ramo anterior tiene id = id_anterior
                updates.push((clave.clone(), id_anterior));
                eprintln!("DEBUG depends: ramo {} (id={}) depende de ramo con id={}", 
                          ramo.nombre, correlativo_actual, id_anterior);
                break;
            }
        }
    }
    
    // Aplicar actualizaciones
    for (clave, id_prev) in updates {
        if let Some(ramo) = ramos_disponibles.get_mut(&clave) {
            ramo.codigo_ref = Some(id_prev);
        }
    }
    
    Ok(ramos_disponibles)
}

