// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica

use std::collections::HashMap;
use std::error::Error;
use crate::models::{Seccion, RamoDisponible};
// ahora puedes llamar: extract::extract_data(...), clique::get_clique_with_user_prefs(...), conflict::horarios_tienen_conflicto(...), pert::set_values_recursive...

use super::{pert, extract, clique};
// ahora puedes usar: pert::build_and_run_pert(...), extract::extract_data(...), clique::get_clique_max_pond(...), conflict::horarios_tienen_conflicto...



/// Ejecutar la ruta crítica usando parámetros provistos por el usuario.
///
/// Esta versión acepta un `InputParams` (por ejemplo parseado desde JSON)
/// y devuelve las soluciones producidas por el planner de clique, lo que
/// facilita exponer el resultado vía HTTP o tests.
pub fn ejecutar_ruta_critica_with_params(
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    // Obtener ramos y secciones, delegar en la versión que acepta datos precomputados.
    // Use the malla and optional sheet provided in params to extract data.
    let initial_map: std::collections::HashMap<String, RamoDisponible> = std::collections::HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match extract::extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return Err(e),
    };

    ejecutar_ruta_critica_with_precomputed(lista_secciones, ramos_actualizados, params)
}

/// Ejecutar la ruta crítica cuando ya se tienen `lista_secciones` y `ramos_actualizados`.
/// Esta variante evita volver a leer/extract_data y permite que `mod.rs` haga
/// la preparación (llamadas a `extract`) y luego invoque aquí la ejecución
/// final (lectura de porcentajes + planner que respeta preferencias).
pub fn ejecutar_ruta_critica_with_precomputed(
    lista_secciones: Vec<Seccion>,
    mut ramos_actualizados: HashMap<String, RamoDisponible>,
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    println!("rutacritica::ruta -> ejecutar_ruta_critica_with_precomputed");

    // Validaciones mínimas
    if params.email.trim().is_empty() {
        return Err(Box::<dyn Error>::from("email is required in InputParams"));
    }

    // Intentar leer porcentajes de aprobados desde el archivo garantizado
    // y usarlo para poblar `RamoDisponible.dificultad`.
    let porcentajes_path = "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx";
    if let Ok(pmap) = crate::excel::leer_porcentajes_aprobados(porcentajes_path) {
        // actualizar ramos_actualizados con la dificultad leída
        for (codigo, (porc, _total)) in pmap.into_iter() {
            if let Some(ramo) = ramos_actualizados.get_mut(&codigo) {
                ramo.dificultad = Some(porc);
            }
        }
    }

    // Delegar la construcción y ejecución del PERT al módulo `pert`.
    if let Err(e) = pert::build_and_run_pert(&mut ramos_actualizados, &lista_secciones, &params.malla) {
        return Err(e);
    }

    // Decidir cuál planner usar: si el usuario NO proporcionó preferencias
    // adicionales (solo entregó `ramos_pasados`) usamos la versión sin prefs
    // `get_clique_max_pond`. En caso contrario usamos la variante que respeta
    // preferencias `get_clique_with_user_prefs`.
    let solo_pasados = params.ramos_prioritarios.is_empty()
        && params.horarios_preferidos.is_empty()
        && params.ranking.is_none()
        && params.student_ranking.is_none();

    let soluciones = if solo_pasados {
        clique::get_clique_max_pond(&lista_secciones, &ramos_actualizados)
    } else {
        clique::get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params)
    };

    Ok(soluciones)
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel;
    use petgraph::graph::{NodeIndex, DiGraph};
    use crate::models::PertNode;

    #[test]
    fn test_prereqs_produce_pert_edges() {
        // Obtener la malla por defecto usando el wrapper compat
        let (ramos_map, nombre_malla, malla_leida) = crate::algorithm::get_ramo_critico();
        assert!(malla_leida, "La malla por defecto no fue leída, no se puede ejecutar el test");

        // Leer prerequisitos desde la malla (hojas adicionales)
        let pr_map = match excel::leer_prerequisitos(&nombre_malla) {
            Ok(m) => m,
            Err(e) => panic!("falló leer_prerequisitos para {}: {}", nombre_malla, e),
        };

        // Si no hay prerequisitos, no hay mucho que comprobar; lo permitimos.
        if pr_map.is_empty() {
            eprintln!("Aviso: no se encontraron prerequisitos en la malla '{}', test termina sin errores.", nombre_malla);
            return;
        }

        // Construir grafo y node_map como en la implementación
        let mut pert_graph: DiGraph<PertNode, ()> = DiGraph::new();
        let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

        for (codigo, ramo) in ramos_map.iter() {
            let node = PertNode {
                codigo: codigo.clone(),
                nombre: ramo.nombre.clone(),
                es: None,
                ef: None,
                ls: None,
                lf: None,
                h: None,
            };
            let idx = pert_graph.add_node(node);
            node_map.insert(codigo.clone(), idx);
        }

        // Obtener lista_secciones a partir del archivo de oferta académica (evitar depender de porcentajes)
        // Intentar primero resolver via resolve_datafile_paths, si falla buscar heurísticamente en DATAFILES_DIR.
        let oferta_path = if let Ok((_m, o, _p)) = crate::excel::resolve_datafile_paths(&nombre_malla) {
            o
        } else {
            // heurística: buscar en DATAFILES_DIR un fichero cuyo nombre contenga 'oferta' o 'oa'
            let mut found: Option<std::path::PathBuf> = None;
            let data_dir = std::path::Path::new(crate::excel::DATAFILES_DIR);
            if let Ok(entries) = std::fs::read_dir(data_dir) {
                for e in entries.flatten() {
                    if !e.path().is_file() { continue; }
                    if let Some(name) = e.file_name().to_str() {
                        let lname = name.to_lowercase();
                        if lname.contains("oferta") || lname.contains("oa") || lname.contains("oferta_academica") {
                            found = Some(e.path());
                            break;
                        }
                    }
                }
            }
            found.expect(&format!("No se pudo localizar archivo de Oferta Académica en {}", crate::excel::DATAFILES_DIR))
        };

        let oferta_str = oferta_path.to_str().expect("oferta path no UTF-8");
        let lista_secciones = match crate::excel::leer_oferta_academica_excel(oferta_str) {
            Ok(s) => s,
            Err(_e) => {
                // Fallback: construir secciones a partir de la malla cuando la oferta no está completa
                eprintln!("Aviso: leer_oferta_academica_excel falló para {} — usando nombres desde la malla como fallback.", oferta_str);
                ramos_map.iter().map(|(codigo, ramo)| Seccion {
                    codigo: codigo.clone(),
                    nombre: ramo.nombre.clone(),
                    seccion: String::new(),
                    horario: Vec::new(),
                    profesor: String::new(),
                    codigo_box: String::new(),
                }).collect()
            }
        };

        // Añadir aristas por prerequisitos obtenidos, intentando resolver nombres a códigos si es necesario
        // Resolver un path utilizable para pasar a asignatura_from_nombre.
        // `resolve_datafile_paths` puede fallar si el nombre no existe; en ese
        // caso intentamos buscar en DATAFILES_DIR cualquier fichero que parezca
        // una malla (contenga 'malla') o que coincida con `nombre_malla`.
        let mut malla_path: Option<std::path::PathBuf> = None;
        if let Ok((p, _o, _p)) = crate::excel::resolve_datafile_paths(&nombre_malla) {
            malla_path = Some(p);
        } else {
            let data_dir = std::path::Path::new(crate::excel::DATAFILES_DIR);
            if let Ok(entries) = std::fs::read_dir(data_dir) {
                for e in entries.flatten() {
                    if !e.path().is_file() { continue; }
                    if let Some(name) = e.file_name().to_str() {
                        let lname = name.to_lowercase();
                        if lname.contains("malla") || name == nombre_malla {
                            malla_path = Some(e.path());
                            break;
                        }
                    }
                }
            }
        }

        let malla_path = malla_path.expect(&format!("no se pudo localizar un fichero de malla para {} en {}", nombre_malla, crate::excel::DATAFILES_DIR));

        // Crear mapa normalizado de códigos para matching más flexible
        fn normalize_code(s: &str) -> String {
            s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
        }
        let mut node_map_norm: std::collections::HashMap<String, NodeIndex> = std::collections::HashMap::new();
        for (k, &v) in node_map.iter() {
            node_map_norm.insert(normalize_code(k), v);
        }

        // Mapa auxiliar por nombre humano -> NodeIndex (normalizado) para los casos
        // en que los prerequisitos estén escritos por nombre en la hoja de prereqs.
        let mut name_map_norm: std::collections::HashMap<String, NodeIndex> = std::collections::HashMap::new();
        for seccion in lista_secciones.iter() {
            let key = normalize_code(&seccion.nombre);
            if let Some(&idx) = node_map.get(&seccion.codigo) {
                name_map_norm.insert(key, idx);
            }
        }

        let mut added_any = false;
        for (codigo, prereqs) in pr_map.iter() {
            for prereq in prereqs.iter() {
                // 1) intento directo por código exacto
                if let (Some(&from), Some(&to)) = (node_map.get(prereq), node_map.get(codigo)) {
                    let _ = pert_graph.add_edge(from, to, ());
                    added_any = true;
                    continue;
                }

                // 1b) intento flexible: normalizar y buscar en node_map_norm
                let prereq_norm = normalize_code(prereq);
                if let Some(&from) = node_map_norm.get(&prereq_norm) {
                    if let Some(&to) = node_map.get(codigo) {
                        let _ = pert_graph.add_edge(from, to, ());
                        added_any = true;
                        continue;
                    }
                }

                // 2) intentar matchear por nombre humano normalizado (p. ej. "Programación Avanzada")
                let prereq_norm_name = normalize_code(prereq);
                if let Some(&from) = name_map_norm.get(&prereq_norm_name) {
                    if let Some(&to) = node_map.get(codigo) {
                        let _ = pert_graph.add_edge(from, to, ());
                        added_any = true;
                        continue;
                    }
                }

                // 3) si no hay match directo por nombre, intentar mapear nombre -> asignatura (código) mediante la utilidad
                if let Ok(Some(asig)) = crate::excel::asignatura_from_nombre(&malla_path, prereq) {
                    // normalizar el código obtenido y buscar en node_map_norm
                    let asig_norm = normalize_code(&asig);
                    if let Some(&from) = node_map_norm.get(&asig_norm) {
                        if let Some(&to) = node_map.get(codigo) {
                            let _ = pert_graph.add_edge(from, to, ());
                            added_any = true;
                            continue;
                        }
                    }
                }

                // 4) como último intento, escanear otros archivos en DATAFILES_DIR y preguntar si alguno mapea el nombre a un código
                if let Ok(entries) = std::fs::read_dir(std::path::Path::new(crate::excel::DATAFILES_DIR)) {
                    'outer: for e in entries.flatten() {
                        if !e.path().is_file() { continue; }
                        let path = e.path();
                        if let Ok(Some(asig)) = crate::excel::asignatura_from_nombre(&path, prereq) {
                            let asig_norm = normalize_code(&asig);
                            if let Some(&from) = node_map_norm.get(&asig_norm) {
                                if let Some(&to) = node_map.get(codigo) {
                                    let _ = pert_graph.add_edge(from, to, ());
                                    added_any = true;
                                    break 'outer;
                                }
                            } else if let Some(&to) = node_map.get(codigo) {
                                // if mapping yields a code that is directly present
                                if let Some(&from_direct) = node_map.get(&asig) {
                                    let _ = pert_graph.add_edge(from_direct, to, ());
                                    added_any = true;
                                    break 'outer;
                                }
                            }
                        }
                    }
                }
            }
        }

        if !added_any {
            // No fallamos: el usuario indicó que trabajemos con los ramos que existan.
            eprintln!("INFO: no se añadieron aristas PERT a partir de los prerequisitos disponibles; ningún prerequisito pudo resolverse con la malla/oferta actual.");
            eprintln!("DEBUG: prereq keys (sample <=20): {:?}", pr_map.keys().take(20).collect::<Vec<_>>());
            let sample_pairs: Vec<(String, String)> = ramos_map.iter().take(20).map(|(k, v)| (k.clone(), v.nombre.clone())).collect();
            eprintln!("DEBUG: malla (codigo -> nombre) sample (<=20): {:?}", sample_pairs);
            eprintln!("DEBUG: malla path used for resolution: {:?}", malla_path);
        }

        // Verificar: para cada pair donde existían nodos, la arista debe encontrarse
        for (codigo, prereqs) in pr_map.iter() {
            for prereq in prereqs.iter() {
                // buscar posibles claves válidas (directa o resolviendo nombre)
                let mut maybe_from: Option<NodeIndex> = None;
                if let Some(&idx) = node_map.get(prereq) { maybe_from = Some(idx); }
                else if let Ok(Some(asig)) = crate::excel::asignatura_from_nombre(&malla_path, prereq) {
                    if let Some(&idx) = node_map.get(&asig) { maybe_from = Some(idx); }
                }

                if let Some(from) = maybe_from {
                    if let Some(&to) = node_map.get(codigo) {
                        assert!(pert_graph.find_edge(from, to).is_some(), "Se esperaba arista {} -> {}", prereq, codigo);
                    }
                }
            }
        }
    }
}
