// Funciones para leer archivos Excel

use std::collections::HashMap;
use calamine::{Reader, Xlsx, open_workbook, Data};
use crate::models::{Seccion, RamoDisponible};
use std::error::Error as StdError;
use std::fs::File;
use zip::ZipArchive;
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;

fn read_sheet_via_zip(path: &str, sheet_name: &str) -> Result<Vec<Vec<String>>, Box<dyn StdError>> {
    // Open xlsx as zip
    let f = File::open(path)?;
    let mut zip = ZipArchive::new(f)?;

    // Load sharedStrings if present
    let mut shared: Vec<String> = Vec::new();
    if let Ok(mut ss) = zip.by_name("xl/sharedStrings.xml") {
        let mut buf = String::new();
        use std::io::Read;
        ss.read_to_string(&mut buf)?;
    let mut reader = XmlReader::from_str(&buf);
    reader.trim_text(true);
    loop {
            match reader.read_event_into(&mut Vec::new()) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"t" => {
                    if let Ok(Event::Text(e)) = reader.read_event_into(&mut Vec::new()) {
                        shared.push(e.unescape().unwrap_or_default().to_string());
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => (),
                Err(e) => return Err(format!("XML parse error in sharedStrings: {}", e).into()),
            }
        }
    }

    // Find sheet xml path: try workbook relations to map name -> target
    let mut sheet_path_opt: Option<String> = None;
    // Read workbook.xml and rels into memory to avoid multiple mutable borrows of zip
    let mut workbook_xml_buf = None;
    if let Ok(mut wb_xml) = zip.by_name("xl/workbook.xml") {
        let mut buf = String::new(); use std::io::Read; wb_xml.read_to_string(&mut buf)?; workbook_xml_buf = Some(buf);
    }
    let mut workbook_rels_buf = None;
    if let Ok(mut rels) = zip.by_name("xl/_rels/workbook.xml.rels") {
        let mut buf = String::new(); use std::io::Read; rels.read_to_string(&mut buf)?; workbook_rels_buf = Some(buf);
    }

    if let Some(wb_buf) = workbook_xml_buf {
        let mut reader = XmlReader::from_str(&wb_buf);
        reader.trim_text(true);
        loop {
            match reader.read_event_into(&mut Vec::new()) {
                Ok(Event::Start(ref e)) if e.name().as_ref() == b"sheet" => {
                    // read attributes
                    let mut name_attr: Option<String> = None;
                    let mut rid_attr: Option<String> = None;
                    for a in e.attributes().with_checks(false) {
                        if let Ok(attr) = a {
                            if attr.key.as_ref() == b"name" { name_attr = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                            if attr.key.as_ref() == b"r:id" { rid_attr = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                        }
                    }
                    if let (Some(n), Some(r)) = (name_attr, rid_attr) {
                        if n == sheet_name {
                            if let Some(rels_buf) = &workbook_rels_buf {
                                let mut rreader = XmlReader::from_str(&rels_buf);
                                rreader.trim_text(true);
                                loop {
                                    match rreader.read_event_into(&mut Vec::new()) {
                                        Ok(Event::Start(ref re)) if re.name().as_ref() == b"Relationship" => {
                                            let mut id = None; let mut target = None;
                                            for a in re.attributes().with_checks(false) {
                                                if let Ok(attr) = a {
                                                    if attr.key.as_ref() == b"Id" { id = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                                                    if attr.key.as_ref() == b"Target" { target = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                                                }
                                            }
                                            if let (Some(i), Some(t)) = (id, target) {
                                                if i == r { sheet_path_opt = Some(format!("xl/{}", t)); break; }
                                            }
                                        }
                                        Ok(Event::Eof) => break,
                                        Ok(_) => (),
                                        Err(e) => return Err(format!("XML parse error in workbook rels: {}", e).into()),
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Ok(_) => (),
                Err(e) => return Err(format!("XML parse error in workbook: {}", e).into()),
            }
        }
    }

    // If not found via workbook.xml, try common path
    if sheet_path_opt.is_none() {
        // try sheet1.xml
        sheet_path_opt = Some("xl/worksheets/sheet1.xml".to_string());
    }

    let sheet_path = sheet_path_opt.ok_or_else(|| "No sheet path found".to_string())?;
    let mut sheet_file = zip.by_name(&sheet_path)
        .map_err(|e| format!("No sheet file {} in archive: {}", sheet_path, e))?;
    let mut sheet_buf = String::new();
    use std::io::Read;
    sheet_file.read_to_string(&mut sheet_buf)?;

    // parse sheet xml rows
    let mut reader = XmlReader::from_str(&sheet_buf);
    reader.trim_text(true);
    let mut rows: Vec<Vec<String>> = Vec::new();

    let mut in_row = false;
    let mut curr_row_cells: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
    let mut _curr_row_idx: usize = 0;
    loop {
        match reader.read_event_into(&mut Vec::new()) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"row" => {
                        in_row = true; curr_row_cells.clear(); _curr_row_idx = 0;
                        for a in e.attributes().with_checks(false) {
                            if let Ok(attr) = a {
                                if attr.key.as_ref() == b"r" {
                                    _curr_row_idx = attr.unescape_value().unwrap_or_default().parse::<usize>().unwrap_or(0);
                                }
                            }
                        }
                    }
                    b"c" if in_row => {
                        // cell start; get r and t
                        let mut cell_ref = None; let mut cell_t = None;
                        for a in e.attributes().with_checks(false) {
                            if let Ok(attr) = a {
                                if attr.key.as_ref() == b"r" { cell_ref = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                                if attr.key.as_ref() == b"t" { cell_t = Some(attr.unescape_value().unwrap_or_default().to_string()); }
                            }
                        }
                        // read until <v>
                        let mut val = String::new();
                        loop {
                            match reader.read_event_into(&mut Vec::new()) {
                                Ok(Event::Start(ref ev)) if ev.name().as_ref() == b"v" => {
                                    if let Ok(Event::Text(t)) = reader.read_event_into(&mut Vec::new()) {
                                        val = t.unescape().unwrap_or_default().to_string();
                                    }
                                }
                                Ok(Event::End(ref ev)) if ev.name().as_ref() == b"c" => break,
                                Ok(Event::Eof) => break,
                                Ok(_) => (),
                                Err(e) => return Err(format!("XML parse error in sheet cells: {}", e).into()),
                            }
                        }
                        if let Some(cref) = cell_ref {
                            // column letters part (e.g., A12 -> A)
                            let col_letters: String = cref.chars().take_while(|c| c.is_alphabetic()).collect();
                            let col_idx = column_letters_to_index(&col_letters);
                            let cell_value = if let Some(t) = cell_t {
                                if t == "s" {
                                    let idx = val.parse::<usize>().unwrap_or(0);
                                    shared.get(idx).cloned().unwrap_or_default()
                                } else { val.clone() }
                            } else { val.clone() };
                            curr_row_cells.insert(col_idx, cell_value);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"row" => {
                // flush row
                let max_col = curr_row_cells.keys().cloned().max().unwrap_or(0);
                let mut row_vec = Vec::new();
                for c in 1..=max_col {
                    row_vec.push(curr_row_cells.remove(&c).unwrap_or_default());
                }
                rows.push(row_vec);
                in_row = false;
            }
            Ok(Event::Eof) => break,
            Ok(_) => (),
            Err(e) => return Err(format!("XML parse error in sheet: {}", e).into()),
        }
    }

    Ok(rows)
}

fn column_letters_to_index(s: &str) -> usize {
    let mut acc = 0usize;
    for ch in s.chars() {
        if ch.is_ascii_alphabetic() {
            acc = acc * 26 + ((ch.to_ascii_uppercase() as u8 - b'A') as usize + 1);
        }
    }
    acc
}

// Función para leer Excel de malla curricular
pub fn leer_malla_excel(nombre_archivo: &str) -> Result<HashMap<String, RamoDisponible>, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(nombre_archivo)?;
    let mut ramos_disponibles = HashMap::new();
    
    // Obtener la primera hoja disponible en lugar de buscar "Sheet1" específicamente
    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }
    
    let primera_hoja = &sheet_names[0];
    println!("Leyendo hoja: {}", primera_hoja);
    
    let range = workbook.worksheet_range(primera_hoja)?;
    
    // Iterar sobre las filas (asumiendo que la primera fila son headers)
    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 { continue; } // Saltar header
        
        // Extraer datos de las columnas
        let codigo = match row.get(0) {
            Some(Data::String(s)) => s.clone(),
            Some(Data::Float(f)) => f.to_string(),
            Some(Data::Int(i)) => i.to_string(),
            Some(Data::Bool(b)) => b.to_string(),
            Some(Data::Empty) => "".to_string(),
            Some(Data::Error(_)) => "".to_string(),
            Some(Data::DateTime(d)) => d.to_string(),
            Some(Data::DateTimeIso(d)) => d.clone(),
            Some(Data::DurationIso(d)) => d.clone(),
            None => "".to_string(),
        };
        let nombre = match row.get(1) {
            Some(Data::String(s)) => s.clone(),
            Some(Data::Float(f)) => f.to_string(),
            Some(Data::Int(i)) => i.to_string(),
            Some(Data::Bool(b)) => b.to_string(),
            Some(Data::Empty) => "".to_string(),
            Some(Data::Error(_)) => "".to_string(),
            Some(Data::DateTime(d)) => d.to_string(),
            Some(Data::DateTimeIso(d)) => d.clone(),
            Some(Data::DurationIso(d)) => d.clone(),
            None => "".to_string(),
        };
        let correlativo = match row.get(2) {
            Some(Data::Float(f)) => *f as i32,
            Some(Data::Int(i)) => *i as i32,
            Some(Data::String(s)) => s.parse::<i32>().unwrap_or(0),
            Some(Data::Bool(b)) => if *b { 1 } else { 0 },
            Some(Data::Empty) => 0,
            Some(Data::Error(_)) => 0,
            Some(Data::DateTime(_)) => 0,
            Some(Data::DateTimeIso(_)) => 0,
            Some(Data::DurationIso(_)) => 0,
            None => 0,
        };
        let holgura = match row.get(3) {
            Some(Data::Float(f)) => *f as i32,
            Some(Data::Int(i)) => *i as i32,
            Some(Data::String(s)) => s.parse::<i32>().unwrap_or(0),
            Some(Data::Bool(b)) => if *b { 1 } else { 0 },
            Some(Data::Empty) => 0,
            Some(Data::Error(_)) => 0,
            Some(Data::DateTime(_)) => 0,
            Some(Data::DateTimeIso(_)) => 0,
            Some(Data::DurationIso(_)) => 0,
            None => 0,
        };
        let critico = match row.get(4) {
            Some(Data::String(s)) => s == "true" || s == "True" || s == "TRUE",
            Some(Data::Int(i)) => *i != 0,
            Some(Data::Float(f)) => *f != 0.0,
            Some(Data::Bool(b)) => *b,
            Some(Data::Empty) => false,
            Some(Data::Error(_)) => false,
            Some(Data::DateTime(_)) => false,
            Some(Data::DateTimeIso(_)) => false,
            Some(Data::DurationIso(_)) => false,
            None => false,
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

/// Leer fichero de porcentajes de aprovados. Se espera una hoja con columnas 'Codigo' y 'Porcentaje'
/// Leer porcentajes/aprobados. Devuelve un mapa codigo -> (A, n) donde
/// A = Est.Aprobados (o porcentaje asumido), n = Est.Total (o 100 si no hay total)
pub fn leer_porcentajes_aprobados(path: &str) -> Result<HashMap<String, (f64, f64)>, Box<dyn std::error::Error>> {
    let mut res: HashMap<String, (f64, f64)> = HashMap::new();

    // helper to convert Data -> String
    let data_to_string = |d: &Data| -> String {
        match d {
            Data::String(s) => s.trim().to_string(),
            Data::Float(f) => f.to_string(),
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
            Data::Empty => "".to_string(),
            Data::Error(_) => "".to_string(),
            Data::DateTime(s) => s.to_string(),
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
        }
    };

    // Intentar con calamine primero
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let sheet_names = workbook.sheet_names().to_owned();
        if !sheet_names.is_empty() {
            let primera = &sheet_names[0];
            if let Ok(range) = workbook.worksheet_range(primera) {
                let mut rows_iter = range.rows();
                // determinar headers
                if let Some(header_row) = rows_iter.next() {
                    let headers: Vec<String> = header_row.iter().map(|c| data_to_string(c).to_lowercase()).collect();
                    // buscar índices
                    let mut idx_codigo: usize = 0;
                    let mut idx_aprobados: Option<usize> = None;
                    let mut idx_total: Option<usize> = None;
                    let mut idx_porcentaje: Option<usize> = None;
                    for (i, h) in headers.iter().enumerate() {
                        let h_trim = h.trim();
                        if h_trim.contains("codigo") || h_trim == "ramo" || h_trim == "asignatura" || h_trim == "codigo ramo" {
                            idx_codigo = i;
                        }
                        if h_trim.contains("aprob") || h_trim.contains("est.aprob") || h_trim == "est.aprobados" || h_trim == "aprobados" {
                            idx_aprobados = Some(i);
                        }
                        if h_trim.contains("total") || h_trim.contains("est.total") || h_trim == "est.total" {
                            idx_total = Some(i);
                        }
                        if h_trim.contains("porcentaje") || h_trim.contains("%") {
                            idx_porcentaje = Some(i);
                        }
                    }

                    for row in rows_iter {
                        let codigo = data_to_string(row.get(idx_codigo).unwrap_or(&Data::Empty));
                        if codigo.trim().is_empty() { continue; }

                        // prefer A/n if available
                        if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                            let a_s = data_to_string(row.get(ai).unwrap_or(&Data::Empty));
                            let n_s = data_to_string(row.get(ni).unwrap_or(&Data::Empty));
                            if let (Ok(a), Ok(n)) = (a_s.replace(',', "").parse::<f64>(), n_s.replace(',', "").parse::<f64>()) {
                                if n > 0.0 {
                                    res.insert(codigo.clone(), (a, n));
                                    continue;
                                }
                            }
                        }

                        // else, if porcentaje column exists
                        if let Some(pi) = idx_porcentaje {
                            let p_s = data_to_string(row.get(pi).unwrap_or(&Data::Empty));
                            let p_clean = p_s.replace('%', "").replace(',', ".");
                            if let Ok(pv) = p_clean.parse::<f64>() {
                                // store as A = pv, n = 100
                                res.insert(codigo.clone(), (pv, 100.0));
                                continue;
                            }
                        }

                        // fallback: try second column as percentage
                        let second = data_to_string(row.get(1).unwrap_or(&Data::Empty));
                        let s2 = second.replace('%', "").replace(',', ".");
                        if let Ok(pv) = s2.parse::<f64>() {
                            res.insert(codigo.clone(), (pv, 100.0));
                        }
                    }
                }
                return Ok(res);
            }
        }

    // Si falla calamine, intentar fallback ZIP/XML
    match read_sheet_via_zip(path, "Sheet1") {
        Ok(rows) => {
            if rows.is_empty() { return Ok(res); }
            // tratar primera fila como header
            let headers_row = &rows[0];
            let headers: Vec<String> = headers_row.iter().map(|h| h.trim().to_lowercase()).collect();
            let mut idx_codigo: usize = 0;
            let mut idx_aprobados: Option<usize> = None;
            let mut idx_total: Option<usize> = None;
            let mut idx_porcentaje: Option<usize> = None;
            for (i, h) in headers.iter().enumerate() {
                let h_trim = h.as_str();
                if h_trim.contains("codigo") || h_trim == "ramo" || h_trim == "asignatura" { idx_codigo = i; }
                if h_trim.contains("aprob") || h_trim.contains("est.aprob") { idx_aprobados = Some(i); }
                if h_trim.contains("total") || h_trim.contains("est.total") { idx_total = Some(i); }
                if h_trim.contains("porcentaje") || h_trim.contains("%") { idx_porcentaje = Some(i); }
            }

            for (i, row) in rows.iter().enumerate() {
                if i == 0 { continue; }
                let codigo = row.get(idx_codigo).cloned().unwrap_or_default().trim().to_string();
                if codigo.is_empty() { continue; }

                if let (Some(ai), Some(ni)) = (idx_aprobados, idx_total) {
                    let a_s = row.get(ai).cloned().unwrap_or_default();
                    let n_s = row.get(ni).cloned().unwrap_or_default();
                    if let (Ok(a), Ok(n)) = (a_s.replace(',', "").parse::<f64>(), n_s.replace(',', "").parse::<f64>()) {
                        if n > 0.0 { res.insert(codigo.clone(), (a, n)); continue; }
                    }
                }

                if let Some(pi) = idx_porcentaje {
                    let p_s = row.get(pi).cloned().unwrap_or_default();
                    let p_clean = p_s.replace('%', "").replace(',', ".");
                    if let Ok(pv) = p_clean.parse::<f64>() { res.insert(codigo.clone(), (pv, 100.0)); continue; }
                }

                // fallback second column
                let second = row.get(1).cloned().unwrap_or_default();
                let s2 = second.replace('%', "").replace(',', ".");
                if let Ok(pv) = s2.parse::<f64>() { res.insert(codigo.clone(), (pv, 100.0)); }
            }
            return Ok(res);
        }
        Err(e) => return Err(format!("No se pudo leer porcentajes: {}", e).into()),
    }
}

// Función para leer Excel de oferta académica
pub fn leer_oferta_academica_excel(nombre_archivo: &str) -> Result<Vec<Seccion>, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(nombre_archivo)?;
    let mut secciones = Vec::new();
    
    // Obtener la primera hoja disponible
    let sheet_names = workbook.sheet_names().to_owned();
    if sheet_names.is_empty() {
        return Err("No se encontraron hojas en el archivo Excel".into());
    }

    // Construir lista de candidatos: hojas explícitas preferidas + todas las hojas del workbook
    let mut candidates: Vec<String> = vec!["Mi Malla".to_string(), "MiMalla".to_string(), "Mi malla".to_string()];
    for s in sheet_names.iter() {
        if !candidates.contains(s) {
            candidates.push(s.clone());
        }
    }

    // Intentar leer el rango probando cada candidate hasta que uno funcione
    let mut _last_err: Option<String> = None;
    let mut used_sheet: Option<String> = None;
    let mut range_opt = None;
    let mut rows_vec_opt: Option<Vec<Vec<String>>> = None;
    for cand in candidates.iter() {
        match workbook.worksheet_range(cand) {
            Ok(rng) => { range_opt = Some(rng); used_sheet = Some(cand.clone()); break; }
            Err(e) => {
                println!("Calamine no pudo leer la hoja '{}': {:?}", cand, e);
                // intentar fallback por zip/xml
                match read_sheet_via_zip(nombre_archivo, cand) {
                    Ok(rv) if !rv.is_empty() => {
                        println!("Fallback ZIP/XML obtuvo {} filas en la hoja '{}'", rv.len(), cand);
                        rows_vec_opt = Some(rv);
                        used_sheet = Some(cand.clone());
                        break;
                    }
                    Ok(_) => {
                        println!("Fallback ZIP/XML no devolvió filas para '{}'", cand);
                    }
                    Err(err2) => {
                        println!("Fallback ZIP/XML también falló para '{}': {}", cand, err2);
                        _last_err = Some(format!("{}: {} (fallback: {})", cand, e, err2));
                    }
                }
                _last_err = Some(format!("{}: {}", cand, e));
            }
        }
    }

    // If we have a calamine range, use it; otherwise if we have rows_vec, use that
    if let Some(range) = range_opt {
        println!("Leyendo hoja seleccionada: {}", used_sheet.unwrap_or_else(|| "<desconocida>".to_string()));

        // Verificar que el rango tenga datos
        println!("Rango obtenido exitosamente");
        let size = range.get_size();
        println!("Tamaño del rango: {:?}", size);
        if size == (0, 0) {
            return Err("El archivo Excel está vacío".into());
        }
        let height = range.height();
        let width = range.width();
        println!("Dimensiones del Excel: {} filas, {} columnas", height, width);
        if height < 2 {
            return Err("El archivo debe tener al menos 2 filas (header + datos)".into());
        }
        if width < 6 {
            println!("⚠️  Advertencia: El archivo tiene solo {} columnas, se esperaban 6", width);
        }

        // Intentar iterar sobre las filas con manejo de errores
        let rows: Vec<_> = range.rows().collect();
        println!("Total de filas recolectadas: {}", rows.len());

        for (row_idx, row) in rows.iter().enumerate() {
            println!("Procesando fila {}: {:?}", row_idx, row);
            if row_idx == 0 { println!("Headers encontrados: {:?}", row); continue; }
            if row.is_empty() { println!("Fila {} está vacía, saltando", row_idx); continue; }

            let codigo = match row.get(0) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => "".to_string(),
                Some(Data::Error(e)) => { println!("Error en celda [{}][0]: {:?}", row_idx, e); "".to_string() },
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => "".to_string(),
            };
            if codigo.is_empty() { println!("Código vacío en fila {}, saltando", row_idx); continue; }

            let nombre = match row.get(1) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => "Sin nombre".to_string(),
                Some(Data::Error(_)) => "Sin nombre".to_string(),
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => "Sin nombre".to_string(),
            };

            let seccion = match row.get(2) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => "1".to_string(),
                Some(Data::Error(_)) => "1".to_string(),
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => "1".to_string(),
            };

            let horario_str = match row.get(3) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => "".to_string(),
                Some(Data::Error(_)) => "".to_string(),
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => "".to_string(),
            };

            let profesor = match row.get(4) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => "Sin asignar".to_string(),
                Some(Data::Error(_)) => "Sin asignar".to_string(),
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => "Sin asignar".to_string(),
            };

            let codigo_box = match row.get(5) {
                Some(Data::String(s)) => s.clone(),
                Some(Data::Float(f)) => f.to_string(),
                Some(Data::Int(i)) => i.to_string(),
                Some(Data::Bool(b)) => b.to_string(),
                Some(Data::Empty) => {
                    if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() }
                },
                Some(Data::Error(_)) => codigo.clone(),
                Some(Data::DateTime(d)) => d.to_string(),
                Some(Data::DateTimeIso(d)) => d.clone(),
                Some(Data::DurationIso(d)) => d.clone(),
                None => { if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() } },
            };

            let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else {
                horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
            };

            secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
            println!("✅ Sección procesada exitosamente: {} - {} (CódigoBox: {})", codigo, nombre, codigo_box);
        }

        println!("Total de secciones procesadas: {}", secciones.len());
        return Ok(secciones);
    }

    // If we reach here, maybe rows_vec_opt has data from ZIP fallback
    if let Some(rows_vec) = rows_vec_opt {
        println!("Procesando filas obtenidas por fallback ZIP/XML: {} filas", rows_vec.len());
        let height = rows_vec.len();
        let width = rows_vec.iter().map(|r| r.len()).max().unwrap_or(0);
        println!("Dimensiones inferidas: {} filas, {} columnas", height, width);
        if height < 2 { return Err("El archivo debe tener al menos 2 filas (header + datos)".into()); }

        for (row_idx, row) in rows_vec.iter().enumerate() {
            if row_idx == 0 { println!("Headers (fallback): {:?}", row); continue; }
            if row.iter().all(|c| c.trim().is_empty()) { println!("Fila {} vacía (fallback), saltando", row_idx); continue; }

            let codigo = row.get(0).cloned().unwrap_or_default();
            if codigo.trim().is_empty() { println!("Código vacío en fila {} (fallback), saltando", row_idx); continue; }
            let nombre = row.get(1).cloned().unwrap_or_else(|| "Sin nombre".to_string());
            let seccion = row.get(2).cloned().unwrap_or_else(|| "1".to_string());
            let horario_str = row.get(3).cloned().unwrap_or_default();
            let profesor = row.get(4).cloned().unwrap_or_else(|| "Sin asignar".to_string());
            let codigo_box = row.get(5).cloned().unwrap_or_else(|| if codigo.contains('-') { codigo.split('-').next().unwrap_or(&codigo).to_string() } else { codigo.clone() });
            let horario: Vec<String> = if horario_str.is_empty() { vec!["Sin horario".to_string()] } else { horario_str.split(|c| c == ',' || c == ';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect() };

            secciones.push(Seccion { codigo: codigo.clone(), nombre: nombre.clone(), seccion: seccion.clone(), horario, profesor, codigo_box: codigo_box.clone() });
            println!("✅ (fallback) Sección procesada exitosamente: {} - {} (CódigoBox: {})", codigo, nombre, codigo_box);
        }
        println!("Total de secciones procesadas (fallback): {}", secciones.len());
        return Ok(secciones);
    }

    return Err(format!("No se pudo leer ninguna hoja del archivo '{}'. Último error: {}", nombre_archivo, _last_err.unwrap_or_else(|| "sin detalles".to_string())).into());
}