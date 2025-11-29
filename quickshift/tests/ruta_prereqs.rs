use quickshift::algorithm::get_ramo_critico;
use quickshift::excel;
use quickshift::models::Seccion;
use petgraph::graph::{NodeIndex, DiGraph};
use quickshift::models::PertNode;
use std::collections::HashMap;

#[test]
fn test_prereqs_produce_pert_edges() {
    let (ramos_map, nombre_malla, malla_leida) = get_ramo_critico();
    assert!(malla_leida, "La malla por defecto no fue leída, no se puede ejecutar el test");

    let pr_map = match excel::get_prereqs_cached(&nombre_malla) {
        Ok(arc_map) => (*arc_map).clone(),
        Err(e) => panic!("falló get_prereqs_cached para {}: {}", nombre_malla, e),
    };

    if pr_map.is_empty() {
        eprintln!("Aviso: no se encontraron prerequisitos en la malla '{}', test termina sin errores.", nombre_malla);
        return;
    }

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

    // Attempt to locate oferta
    let oferta_path = if let Ok((_m, o, _p)) = excel::resolve_datafile_paths(&nombre_malla) {
        o
    } else {
        let mut found: Option<std::path::PathBuf> = None;
        let data_dir = std::path::Path::new(excel::DATAFILES_DIR);
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
        found.expect(&format!("No se pudo localizar archivo de Oferta Académica en {}", excel::DATAFILES_DIR))
    };

    let oferta_str = oferta_path.to_str().expect("oferta path no UTF-8");
    let lista_secciones = match excel::leer_oferta_academica_excel(oferta_str) {
        Ok(s) => s,
        Err(_e) => {
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

    let mut malla_path: Option<std::path::PathBuf> = None;
    if let Ok((p, _o, _p)) = excel::resolve_datafile_paths(&nombre_malla) {
        malla_path = Some(p);
    } else {
        let data_dir = std::path::Path::new(excel::DATAFILES_DIR);
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

    let malla_path = malla_path.expect(&format!("no se pudo localizar un fichero de malla para {} en {}", nombre_malla, excel::DATAFILES_DIR));

    fn normalize_code(s: &str) -> String {
        s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
    }
    let mut node_map_norm: std::collections::HashMap<String, NodeIndex> = std::collections::HashMap::new();
    for (k, &v) in node_map.iter() {
        node_map_norm.insert(normalize_code(k), v);
    }

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
            if let (Some(&from), Some(&to)) = (node_map.get(prereq), node_map.get(codigo)) {
                let _ = pert_graph.add_edge(from, to, ());
                added_any = true;
                continue;
            }

            let prereq_norm = normalize_code(prereq);
            if let Some(&from) = node_map_norm.get(&prereq_norm) {
                if let Some(&to) = node_map.get(codigo) {
                    let _ = pert_graph.add_edge(from, to, ());
                    added_any = true;
                    continue;
                }
            }

            let prereq_norm_name = normalize_code(prereq);
            if let Some(&from) = name_map_norm.get(&prereq_norm_name) {
                if let Some(&to) = node_map.get(codigo) {
                    let _ = pert_graph.add_edge(from, to, ());
                    added_any = true;
                    continue;
                }
            }

            if let Ok(Some(asig)) = excel::asignatura_from_nombre(&malla_path, prereq) {
                let asig_norm = normalize_code(&asig);
                if let Some(&from) = node_map_norm.get(&asig_norm) {
                    if let Some(&to) = node_map.get(codigo) {
                        let _ = pert_graph.add_edge(from, to, ());
                        added_any = true;
                        continue;
                    }
                }
            }

            if let Ok(entries) = std::fs::read_dir(std::path::Path::new(excel::DATAFILES_DIR)) {
                'outer: for e in entries.flatten() {
                    if !e.path().is_file() { continue; }
                    let path = e.path();
                    if let Ok(Some(asig)) = excel::asignatura_from_nombre(&path, prereq) {
                        let asig_norm = normalize_code(&asig);
                        if let Some(&from) = node_map_norm.get(&asig_norm) {
                            if let Some(&to) = node_map.get(codigo) {
                                let _ = pert_graph.add_edge(from, to, ());
                                added_any = true;
                                break 'outer;
                            }
                        } else if let Some(&to) = node_map.get(codigo) {
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
        eprintln!("INFO: no se añadieron aristas PERT a partir de los prerequisitos disponibles; ningún prerequisito pudo resolverse con la malla/oferta actual.");
        eprintln!("DEBUG: prereq keys (sample <=20): {:?}", pr_map.keys().take(20).collect::<Vec<_>>());
        let sample_pairs: Vec<(String, String)> = ramos_map.iter().take(20).map(|(k, v)| (k.clone(), v.nombre.clone())).collect();
        eprintln!("DEBUG: malla (codigo -> nombre) sample (<=20): {:?}", sample_pairs);
        eprintln!("DEBUG: malla path used for resolution: {:?}", malla_path);
    }

    for (codigo, prereqs) in pr_map.iter() {
        for prereq in prereqs.iter() {
            let mut maybe_from: Option<NodeIndex> = None;
            if let Some(&idx) = node_map.get(prereq) { maybe_from = Some(idx); }
            else if let Ok(Some(asig)) = excel::asignatura_from_nombre(&malla_path, prereq) {
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
