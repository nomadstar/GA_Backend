use std::collections::HashMap;
use crate::models::{Seccion, RamoDisponible};
use crate::excel::{leer_malla_excel, leer_oferta_academica_excel};

pub fn get_ramo_critico() -> (HashMap<String, RamoDisponible>, String, bool) {
    println!("Leyendo ramos críticos desde Excel...");
    let nombre_excel_malla = "MiMalla.xlsx";
    let porcentajes_file = "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx";

    match leer_malla_excel(nombre_excel_malla) {
        Ok(ramos_disponibles) => {
            println!("✅ Datos leídos exitosamente desde {}", nombre_excel_malla);
            // intentar leer porcentajes (no obligatorio)
            let _ = crate::excel::leer_porcentajes_aprobados(porcentajes_file);

            (ramos_disponibles, nombre_excel_malla.to_string(), true)
        }
        Err(e) => {
            println!("⚠️  No se pudo leer el archivo Excel: {}", e);
            println!("Usando datos de ejemplo...");
            let (mapa, nombre) = create_fallback_data(nombre_excel_malla);
            (mapa, nombre, false)
        }
    }
}

pub fn extract_data(
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    _nombre_excel_malla: &str,
) -> (Vec<Seccion>, HashMap<String, RamoDisponible>, bool) {
    println!("Procesando extract_data...");
    let oferta_academica_file = "../RutaCritica/Oferta Academica 2021-1 vacantes 2021-02-04.xlsx";

    match leer_oferta_academica_excel(oferta_academica_file) {
        Ok(mut lista_secciones) => {
            // Normalizar valores si es necesario (omito detalles)
            for s in lista_secciones.iter_mut() {
                // guardamos como estaba la lógica original: placeholder
                let _ = &s.codigo;
            }

            // Filtrar por ramos disponibles
            lista_secciones.retain(|seccion| {
                ramos_disponibles.iter().any(|(_, ramo)| ramo.codigo == seccion.codigo_box)
            });

            println!("✅ Se encontraron {} secciones desde Excel", lista_secciones.len());
            (lista_secciones, ramos_disponibles.clone(), true)
        }
        Err(e) => {
            println!("⚠️  No se pudo leer oferta académica: {}", e);
            println!("Generando datos simulados...");
            let (secs, map) = create_simulated_sections(ramos_disponibles);
            (secs, map, false)
        }
    }
}

fn create_fallback_data(nombre_excel_malla: &str) -> (HashMap<String, RamoDisponible>, String) {
    let mut ramos_disponibles = HashMap::new();

    ramos_disponibles.insert("CIT3313".to_string(), RamoDisponible {
        nombre: "Algoritmos y Programación".to_string(),
        codigo: "CIT3313".to_string(),
        holgura: 0,
        numb_correlativo: 53,
        critico: true,
        codigo_ref: Some("CIT3313".to_string()),
        dificultad: None,
    });

    ramos_disponibles.insert("CIT3211".to_string(), RamoDisponible {
        nombre: "Bases de Datos".to_string(),
        codigo: "CIT3211".to_string(),
        holgura: 0,
        numb_correlativo: 52,
        critico: true,
        codigo_ref: Some("CIT3211".to_string()),
        dificultad: None,
    });

    ramos_disponibles.insert("CIT3413".to_string(), RamoDisponible {
        nombre: "Redes de Computadores".to_string(),
        codigo: "CIT3413".to_string(),
        holgura: 2,
        numb_correlativo: 54,
        critico: false,
        codigo_ref: Some("CIT3413".to_string()),
        dificultad: None,
    });

    ramos_disponibles.insert("CFG-1".to_string(), RamoDisponible {
        nombre: "Curso de Formación General".to_string(),
        codigo: "CFG-1".to_string(),
        holgura: 3,
        numb_correlativo: 10,
        critico: false,
        codigo_ref: Some("CFG-1".to_string()),
        dificultad: None,
    });

    (ramos_disponibles, nombre_excel_malla.to_string())
}

fn create_simulated_sections(ramos_disponibles: &HashMap<String, RamoDisponible>) -> (Vec<Seccion>, HashMap<String, RamoDisponible>) {
    let mut lista_secciones = Vec::new();
    for (codigo_box, ramo) in ramos_disponibles {
        for seccion_num in 1..=2 {
            let s = Seccion {
                codigo: format!("{}-SEC{}", codigo_box, seccion_num),
                codigo_box: codigo_box.clone(),
                nombre: ramo.nombre.clone(),
                seccion: seccion_num.to_string(),
                profesor: format!("Profesor {}", seccion_num),
                horario: vec!["08:00-10:00".to_string()],
            };
            lista_secciones.push(s);
        }
    }
    (lista_secciones, ramos_disponibles.clone())
}
