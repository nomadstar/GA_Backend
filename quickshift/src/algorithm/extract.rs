// Port literal del script Python `RutaCritica/extract_data.py` a Rust.
//
// Este módulo implementa las mismas funciones auxiliares que el script
// original: equivalencia, counters, appendElectivos, secciones_cfg y
// extract_data. Está escrito de manera relativamente literal para facilitar
// la revisión y posteriores refactorizaciones.

use std::collections::HashMap;
use std::error::Error;
use calamine::{open_workbook, Xlsx, Data, Reader};
use crate::models::{Seccion, RamoDisponible};

fn read_sheet_rows(file: &str, sheet_name: &str) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut workbook: Xlsx<_> = open_workbook(file)?;

    // Try the requested sheet first. If it fails with a RangeWithoutRowComponent
    // or the sheet is missing, try a fallback sequence: "MiMalla" then the first sheet.
    let try_sheets = vec![sheet_name.to_string(), "Mi Malla".to_string(), "MiMalla".to_string()];
    let mut last_err: Option<String> = None;

    for candidate in try_sheets.iter() {
        match workbook.worksheet_range(candidate) {
            Ok(range) => {
                println!("Leyendo hoja: {} (usada como '{}')", candidate, sheet_name);
                return rows_from_range(range);
            }
            Err(e) => {
                println!("No se pudo leer la hoja '{}': {:?}", candidate, e);
                last_err = Some(format!("{}: {}", candidate, e));
            }
        }
    }

    // Fallback to the first sheet available
    let sheet_names = workbook.sheet_names().to_owned();
    if !sheet_names.is_empty() {
        // Try every sheet and pick the first with a non-empty range (height >= 2)
        for name in sheet_names.iter() {
            match workbook.worksheet_range(name) {
                Ok(rng) => {
                    let size = rng.get_size();
                    if size.0 >= 2 {
                        println!("Seleccionando hoja válida: {} ({} filas, {} cols)", name, size.0, size.1);
                        return rows_from_range(rng);
                    } else {
                        println!("Hoja {} descartada por tamaño: {:?}", name, size);
                    }
                }
                Err(e) => {
                    println!("No se pudo leer la hoja '{}': {:?}", name, e);
                    last_err = Some(format!("{}: {}", name, e));
                }
            }
        }
    }

    Err(format!("No se pudo leer ninguna hoja del archivo '{}'. Último error: {}", file, last_err.unwrap_or_else(|| "sin detalles".to_string())).into())
}

fn rows_from_range(range: calamine::Range<calamine::Data>) -> Result<Vec<Vec<String>>, Box<dyn Error>> {
    let mut rows_out: Vec<Vec<String>> = Vec::new();
    for row in range.rows() {
        let mut out_row: Vec<String> = Vec::new();
        for cell in row {
            let s = match cell {
                Data::String(s) => s.clone(),
                Data::Float(f) => f.to_string(),
                Data::Int(i) => i.to_string(),
                Data::Bool(b) => b.to_string(),
                Data::Empty => "".to_string(),
                Data::Error(_) => "".to_string(),
                Data::DateTime(d) => d.to_string(),
                Data::DateTimeIso(d) => d.clone(),
                Data::DurationIso(d) => d.clone(),
            };
            out_row.push(s);
        }
        rows_out.push(out_row);
    }
    Ok(rows_out)
}

// equivalencia: mirror del python
pub fn equivalencia(
    ramos_disponibles: &mut HashMap<String, RamoDisponible>,
    oferta_academica: &Vec<Vec<String>>,
    equivalencia_ramos: &Vec<Vec<String>>,
) {
    // construir índice de equivalencias (col0 -> col1)
    let mut eq_map: HashMap<String, String> = HashMap::new();
    for row in equivalencia_ramos.iter() {
        if row.len() >= 2 {
            eq_map.insert(row[0].clone(), row[1].clone());
        }
    }

    for key in ramos_disponibles.clone().keys() {

        // verificar si aparece en la columna 1 de oferta_academica
        let mut found_in_oferta = false;
        for row in oferta_academica.iter() {
            if row.len() > 1 && row[1] == *key {
                found_in_oferta = true;
                break;
            }
        }

        if !found_in_oferta {
            if let Some(eq) = eq_map.get(key) {
                if let Some(r) = ramos_disponibles.get_mut(key) {
                    r.codigo_ref = Some(eq.clone());
                }
            } else {
                if let Some(r) = ramos_disponibles.get_mut(key) {
                    r.codigo_ref = Some(key.clone());
                }
            }
        } else {
            // si está en oferta_academica, intentar lógica análoga a python
            // buscamos la fila y vemos el índice 7 (pos 7) si es string
            let mut pos_cod_ramo = None;
            for (i, row) in oferta_academica.iter().enumerate() {
                if row.len() > 1 && row[1] == *key {
                    pos_cod_ramo = Some(i);
                    break;
                }
            }

            if let Some(pos) = pos_cod_ramo {
                let val7 = oferta_academica[pos].get(7).cloned().unwrap_or_default();
                if !val7.is_empty() {
                    if let Some(r) = ramos_disponibles.get_mut(key) {
                        r.codigo_ref = Some(key.clone());
                    }
                } else if let Some(eq) = eq_map.get(key) {
                    if let Some(r) = ramos_disponibles.get_mut(key) {
                        r.codigo_ref = Some(eq.clone());
                    }
                } else {
                    if let Some(r) = ramos_disponibles.get_mut(key) {
                        r.codigo_ref = Some(key.clone());
                    }
                }
            }
        }
    }
}

pub fn counter_cfg_malla(malla_alumno: &Vec<Vec<String>>) -> usize {
    let mut count = 0usize;
    for row in malla_alumno.iter() {
        if row.len() > 1 {
            let cod = &row[1];
            if cod.len() >= 3 && &cod[0..3] == "CFG" {
                count += 1;
            }
        }
    }
    count
}

pub fn counter_cfg_aprobados(ramos_aprobados: &Vec<Vec<String>>) -> (usize, Vec<String>) {
    let mut count = 0usize;
    let mut cfgs: Vec<String> = Vec::new();
    for row in ramos_aprobados.iter() {
        if row.len() > 1 {
            let cod = &row[1];
            if cod.len() >= 3 && &cod[0..3] == "CFG" {
                count += 1;
                cfgs.push(cod.clone());
            }
        }
    }
    (count, cfgs)
}

fn safe_parse_seccion(s: &str) -> i32 {
    if s.len() >= 10 {
        // python took chars [8] and [9]
        let part = &s[8..10];
        part.parse::<i32>().unwrap_or(0)
    } else if s.len() > 8 {
        let ch = &s[8..9];
        ch.parse::<i32>().unwrap_or(0)
    } else { 0 }
}

pub fn secciones_cfg(
    lista_secciones: &mut Vec<Seccion>,
    cant_cfg_malla: usize,
    cant_cfg_aprobados: usize,
        _cfg_aprobados: &Vec<String>,
) -> Result<(), Box<dyn Error>> {
    // Leer oferta de CFG (archivo local en RutaCritica)
    let oferta_file = "RutaCritica/CURSOS-DE-FORMACIÓN-GENERAL.xlsx";
    let rows = read_sheet_rows(oferta_file, "Sheet1")?;

    for z in (cant_cfg_aprobados + 1)..=cant_cfg_malla {
        for elem in rows.iter() {
            // replicar la lógica de python: elem[5] tipo y horarios en elem[6]
            if elem.len() > 6 {
                let tipo = elem[5].clone();
                if !tipo.is_empty() && (tipo.starts_with('C') || tipo.starts_with('B')) {
                    // procesar horarios similares al python
                    let horario_raw = elem[6].clone();
                    let mut aux_horario: Vec<String> = Vec::new();
                    let parts: Vec<&str> = horario_raw.split_whitespace().collect();
                    match parts.len() {
                        5 => {
                            aux_horario.push(format!("{} {}", parts[0], parts[2]));
                            aux_horario.push(format!("{} {}", parts[1], parts[2]));
                        }
                        4 => {
                            aux_horario.push(format!("{} {}", parts[0], parts[1]));
                        }
                        8 => {
                            aux_horario.push(format!("{} {}", parts[0], parts[1]));
                            aux_horario.push(format!("{} {}", parts[4], parts[5]));
                        }
                        _ => {}
                    }

                    let codigo = elem.get(10).cloned().unwrap_or_default();
                    let nombre = elem.get(1).cloned().unwrap_or_default();
                    let seccion = if let Some(sv) = elem.get(4) { safe_parse_seccion(sv) } else { 0 };
                    let profesor = elem.get(7).cloned().unwrap_or_default();

                    if !codigo.is_empty() {
                        let aux_box = format!("CFG-{}", z);
                        // check duplicates
                        let exists = lista_secciones.iter().any(|ls| ls.codigo == codigo && ls.codigo_box == aux_box);
                        if !exists {
                            lista_secciones.push(Seccion {
                                codigo: codigo.clone(),
                                nombre: nombre.clone(),
                                seccion: seccion.to_string(),
                                horario: aux_horario.clone(),
                                profesor: profesor.clone(),
                                codigo_box: aux_box.clone(),
                            });
                            if lista_secciones.len() >= 130 {
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn counter_electivos_malla(malla_alumno: &Vec<Vec<String>>) -> (usize, usize) {
    let mut inf = 0usize;
    let mut tel = 0usize;
    for row in malla_alumno.iter() {
        if row.len() > 1 {
            let cod = &row[1];
            if cod.len() >= 5 {
                let pref = &cod[0..5];
                if pref == "CIT33" { inf += 1; }
                else if pref == "CIT34" { tel += 1; }
            }
        }
    }
    (inf, tel)
}

pub fn counter_electivos_aprobados(ramos_aprobados: &Vec<Vec<String>>) -> (usize, usize, Vec<String>) {
    let mut inf = 0usize;
    let mut tel = 0usize;
    let mut electivos: Vec<String> = Vec::new();
    for row in ramos_aprobados.iter() {
        if row.len() > 1 {
            let cod = &row[1];
            if cod.len() >= 5 {
                let pref = &cod[0..5];
                if pref == "CIT33" { inf += 1; electivos.push(cod.clone()); }
                else if pref == "CIT34" { tel += 1; electivos.push(cod.clone()); }
            }
        }
    }
    (inf, tel, electivos)
}

pub fn append_electivos(
    lista_secciones: &mut Vec<Seccion>,
    oferta_academica: &Vec<Vec<String>>,
    ramos_aprobados: &Vec<Vec<String>>,
    electivos_malla: &Vec<Vec<String>>,
    cant_elect_inf_malla: usize,
    cant_elect_tel_malla: usize,
    count_electivos_inf_aprobados: usize,
    count_electivos_tel_aprobados: usize,
        _electivos_aprobados: &Vec<String>,
) {
    // Construir electivos_can_take
    let mut electivos_can_take: Vec<String> = Vec::new();
    for em in electivos_malla.iter() {
        if em.len() > 4 {
            let requisitos = em[4].split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>();
            let mut ok = true;
            for req in requisitos.iter() {
                let found = ramos_aprobados.iter().any(|ra| ra.get(1).map(|s| s==req).unwrap_or(false));
                if !found { ok = false; break; }
            }
            if ok { electivos_can_take.push(em[1].clone()); }
        }
    }

    // Añadir electivos INF
    for z in count_electivos_inf_aprobados..cant_elect_inf_malla {
        for elem in oferta_academica.iter() {
            if elem.len() > 7 {
                    let tipo = elem[5].clone();
                let mut aux_horario: Vec<String> = Vec::new();
                if !tipo.is_empty() {
                    if tipo.starts_with('C') {
                        let parts: Vec<&str> = elem[7].split_whitespace().collect();
                        match parts.len() {
                            5 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[2]));
                                aux_horario.push(format!("{} {}", parts[1], parts[2]));
                            }
                            4 => { aux_horario.push(format!("{} {}", parts[0], parts[1])); }
                            6 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[3]));
                                aux_horario.push(format!("{} {}", parts[1], parts[3]));
                                aux_horario.push(format!("{} {}", parts[2], parts[3]));
                            }
                            8 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[1]));
                                aux_horario.push(format!("{} {}", parts[4], parts[5]));
                            }
                            _ => {}
                        }

                        let codigo = elem.get(4).cloned().unwrap_or_default();
                        let cod_ramo = elem.get(1).cloned().unwrap_or_default();
                        let nombre = elem.get(2).cloned().unwrap_or_default();
                        let vac_free = elem.get(12).cloned().unwrap_or_default();
                        let seccion = if let Some(sv) = elem.get(3) { safe_parse_seccion(sv) } else { 0 };
                        let profesor = elem.get(9).cloned().unwrap_or_default();

                        if electivos_can_take.contains(&cod_ramo) {
                            if vac_free.parse::<i32>().unwrap_or(0) > 0 {
                                if cod_ramo.starts_with("CIT33") {
                                    let aux_box = format!("CIT331{}", z);
                                    let exists = lista_secciones.iter().any(|ls| ls.codigo == codigo && ls.codigo_box == aux_box);
                                    if !exists && seccion != 99 {
                                        lista_secciones.push(Seccion {
                                            codigo: codigo.clone(),
                                            nombre: nombre.clone(),
                                            seccion: seccion.to_string(),
                                            horario: aux_horario.clone(),
                                            profesor: profesor.clone(),
                                            codigo_box: aux_box.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    } else if tipo.starts_with('A') || tipo.starts_with('L') {
                        let aux = elem[7].split_whitespace().collect::<Vec<_>>();
                        if !aux.is_empty() {
                            // ayudantía/lab: tomar primer módulo
                        }
                    }
                }
            }
        }
    }

    // La parte para CIT34 es análoga; por brevedad la repetimos con pequeña adaptación
    for z in count_electivos_tel_aprobados..cant_elect_tel_malla {
        for elem in oferta_academica.iter() {
            if elem.len() > 7 {
                let tipo = elem[5].clone();
                let mut aux_horario: Vec<String> = Vec::new();
                if !tipo.is_empty() {
                    if tipo.starts_with('C') {
                        let parts: Vec<&str> = elem[7].split_whitespace().collect();
                        match parts.len() {
                            5 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[2]));
                                aux_horario.push(format!("{} {}", parts[1], parts[2]));
                            }
                            4 => { aux_horario.push(format!("{} {}", parts[0], parts[1])); }
                            6 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[3]));
                                aux_horario.push(format!("{} {}", parts[1], parts[3]));
                                aux_horario.push(format!("{} {}", parts[2], parts[3]));
                            }
                            8 => {
                                aux_horario.push(format!("{} {}", parts[0], parts[1]));
                                aux_horario.push(format!("{} {}", parts[4], parts[5]));
                            }
                            _ => {}
                        }

                        let codigo = elem.get(4).cloned().unwrap_or_default();
                        let cod_ramo = elem.get(1).cloned().unwrap_or_default();
                        let nombre = elem.get(2).cloned().unwrap_or_default();
                        let vac_free = elem.get(12).cloned().unwrap_or_default();
                        let seccion = if let Some(sv) = elem.get(3) { safe_parse_seccion(sv) } else { 0 };
                        let profesor = elem.get(9).cloned().unwrap_or_default();

                        if electivos_can_take.contains(&cod_ramo) {
                            if vac_free.parse::<i32>().unwrap_or(0) > 0 {
                                if cod_ramo.starts_with("CIT34") {
                                    let aux_box = format!("CIT341{}", z);
                                    let exists = lista_secciones.iter().any(|ls| ls.codigo == codigo && ls.codigo_box == aux_box);
                                    if !exists && seccion != 99 {
                                        lista_secciones.push(Seccion {
                                            codigo: codigo.clone(),
                                            nombre: nombre.clone(),
                                            seccion: seccion.to_string(),
                                            horario: aux_horario.clone(),
                                            profesor: profesor.clone(),
                                            codigo_box: aux_box.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Port literal de la función `extract_data` del script Python.
/// Devuelve (lista_secciones, ramos_disponibles_actualizado)
pub fn extract_data(
    mut ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    let mut lista_secciones: Vec<Seccion> = Vec::new();

    // Leer oferta academica (archivo local en repo RutaCritica)
    let oferta_file = "RutaCritica/Oferta Academica 2021-1 vacantes 2021-02-04.xlsx";
    let oferta_academica = read_sheet_rows(oferta_file, "Sheet1")?;

    // Electivos y equivalencias desde la malla
    let electivos_malla = read_sheet_rows(nombre_excel_malla, "Electivos")?;
    let equivalencia_ramos = read_sheet_rows(nombre_excel_malla, "Equivalencias")?;

    // Ramos aprobados y malla alumno
    let ramos_aprobados = read_sheet_rows("RutaCritica/MiMalla.xlsx", "MiMalla")?;
    let malla_alumno = read_sheet_rows(nombre_excel_malla, "MiMalla")?;

    // Aplicar equivalencias
    equivalencia(&mut ramos_disponibles, &oferta_academica, &equivalencia_ramos);

    // Procesar filas de oferta para generar lista_secciones (similar al script python)
    for elem in oferta_academica.iter() {
        if elem.len() == 0 { continue; }

        // si elem[5] es string
        let tipo = elem.get(5).cloned().unwrap_or_default();
        let mut aux_horario: Vec<String> = Vec::new();

        if !tipo.is_empty() {
            if tipo.starts_with('C') {
                // Catedra
                let horario_raw = elem.get(7).cloned().unwrap_or_default();
                let parts: Vec<&str> = horario_raw.split_whitespace().collect();
                match parts.len() {
                    5 => {
                        aux_horario.push(format!("{} {}", parts[0], parts[2]));
                        aux_horario.push(format!("{} {}", parts[1], parts[2]));
                    }
                    4 => { aux_horario.push(format!("{} {}", parts[0], parts[1])); }
                    6 => {
                        aux_horario.push(format!("{} {}", parts[0], parts[3]));
                        aux_horario.push(format!("{} {}", parts[1], parts[3]));
                        aux_horario.push(format!("{} {}", parts[2], parts[3]));
                    }
                    8 => {
                        aux_horario.push(format!("{} {}", parts[0], parts[1]));
                        aux_horario.push(format!("{} {}", parts[4], parts[5]));
                    }
                    _ => {}
                }

                let codigo = elem.get(4).cloned().unwrap_or_default();
                let cod_ramo = elem.get(1).cloned().unwrap_or_default();
                let nombre = elem.get(2).cloned().unwrap_or_default();
                let vac_free = elem.get(12).cloned().unwrap_or_default();
                let seccion = if let Some(sv) = elem.get(3) { safe_parse_seccion(sv) } else { 0 };
                let profesor = elem.get(9).cloned().unwrap_or_default();

                // buscar coincidencias en ramos_disponibles por codigo_ref
                for (j, _v) in ramos_disponibles.clone().iter() {
                    let cod_ref = ramos_disponibles.get(j).and_then(|r| r.codigo_ref.clone()).unwrap_or(j.clone());
                    if cod_ramo == cod_ref && vac_free.parse::<i32>().unwrap_or(0) > 0 {
                        let alfa_box = j.clone();
                        let exists = lista_secciones.iter().any(|ls| ls.codigo == codigo && ls.codigo_box == alfa_box);
                        if !exists && seccion != 99 {
                            lista_secciones.push(Seccion {
                                codigo: codigo.clone(),
                                nombre: nombre.clone(),
                                seccion: seccion.to_string(),
                                horario: aux_horario.clone(),
                                profesor: profesor.clone(),
                                codigo_box: alfa_box.clone(),
                            });
                        }
                    }
                }
            } else if tipo.starts_with('A') || tipo.starts_with('L') {
                // Ayudantía o Laboratorio: guardar primer módulo
                let horario_raw = elem.get(7).cloned().unwrap_or_default();
                let parts: Vec<&str> = horario_raw.split_whitespace().collect();
                if parts.len() >= 2 {
                    let aux = format!("{} {}", parts[0], parts[1]);
                    aux_horario.push(aux);
                }
            }
        } else {
            // Cuando tipo no es string, en python se itera por ramos_disponibles
            for j in ramos_disponibles.clone().keys() {
                let cod_ref = ramos_disponibles.get(j).and_then(|r| r.codigo_ref.clone()).unwrap_or(j.clone());
                let cod_ramo = elem.get(1).cloned().unwrap_or_default();
                let codigo = elem.get(0).cloned().unwrap_or_default();
                let nombre = elem.get(1).cloned().unwrap_or_default();
                let seccion = if let Some(sv) = elem.get(3) { safe_parse_seccion(sv) } else { 0 };
                let profesor = elem.get(7).cloned().unwrap_or_default();
                let vac_free = elem.get(12).cloned().unwrap_or_default();

                if cod_ramo == cod_ref && vac_free.parse::<i32>().unwrap_or(0) > 0 {
                    let exists = lista_secciones.iter().any(|ls| ls.codigo == codigo && ls.codigo_box == j.clone());
                    if !exists && seccion != 99 {
                        lista_secciones.push(Seccion {
                            codigo: codigo.clone(),
                            nombre: nombre.clone(),
                            seccion: seccion.to_string(),
                            horario: aux_horario.clone(),
                            profesor: profesor.clone(),
                            codigo_box: j.clone(),
                        });
                    }
                }
            }
        }
    }

    // Electivos
    let (cant_elect_inf_malla, cant_elect_tel_malla) = counter_electivos_malla(&malla_alumno);
    let (count_electivos_inf_aprobados, count_electivos_tel_aprobados, electivos_aprobados) = counter_electivos_aprobados(&ramos_aprobados);

    append_electivos(
        &mut lista_secciones,
        &oferta_academica,
        &ramos_aprobados,
        &electivos_malla,
        cant_elect_inf_malla,
        cant_elect_tel_malla,
        count_electivos_inf_aprobados,
        count_electivos_tel_aprobados,
        &electivos_aprobados,
    );

    let cant_cfg_malla = counter_cfg_malla(&malla_alumno);
    let (cant_cfg_aprobados, cfg_aprobados) = counter_cfg_aprobados(&ramos_aprobados);

    let _ = secciones_cfg(&mut lista_secciones, cant_cfg_malla, cant_cfg_aprobados, &cfg_aprobados);

    // Fin: devolver lista y ramos actualizados
    Ok((lista_secciones, ramos_disponibles))
}

/// Helper público que devuelve el set inicial de ramos disponibles y el nombre
/// de la malla por defecto. En la versión mínima devolvemos una estructura
/// vacía cuando no existe una fuente persistente; esto permite a los callers
/// invocar la extracción y la ejecución sin romper la API.
pub fn get_ramo_critico() -> (HashMap<String, RamoDisponible>, String, bool) {
    // Comportamiento conservador: no hay datos persistidos en este helper.
    let map: HashMap<String, RamoDisponible> = HashMap::new();
    // Nombre de malla por defecto usado en varios handlers/test
    let nombre_malla = "MiMalla.xlsx".to_string();
    // indicador si la malla fue leída desde disco; false pues no intentamos leer aquí
    let malla_leida = false;
    (map, nombre_malla, malla_leida)
}
