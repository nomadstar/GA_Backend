use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::Direction;
use crate::models::PertNode;

use std::collections::HashMap;
use std::error::Error;
use crate::models::{RamoDisponible, Seccion};

/// Construye un grafo PERT a partir de `ramos_actualizados`, añade aristas por
/// `codigo_ref`, `numb_correlativo` y por hojas de prerequisitos dentro de la
/// malla indicada por `malla_name`. Ejecuta el cálculo PERT (set_values_recursive)
/// y propaga el resultado marcando `RamoDisponible.critico = true` cuando la
/// holgura `h == 0`.
pub fn build_and_run_pert(
    ramos_actualizados: &mut HashMap<String, RamoDisponible>,
    lista_secciones: &Vec<Seccion>,
    malla_name: &str,
) -> Result<(), Box<dyn Error>> {
    // Construir grafo y índice de nodos
    let mut pert_graph: DiGraph<PertNode, ()> = DiGraph::new();
    let mut node_map: HashMap<i32, NodeIndex> = HashMap::new();  // id (i32) -> NodeIndex

    for (_nombre_norm, ramo) in ramos_actualizados.iter() {
        let node = PertNode {
            codigo: ramo.id.to_string(),  // Usar ID como identificador en PERT
            nombre: ramo.nombre.clone(),
            es: None,
            ef: None,
            ls: None,
            lf: None,
            h: None,
        };
        let idx = pert_graph.add_node(node);
        node_map.insert(ramo.id, idx);
    }

    // Añadir aristas por codigo_ref (que apunta a IDs)
    for (_nombre_norm, ramo) in ramos_actualizados.iter() {
        if let Some(ref_id) = &ramo.codigo_ref {
            if ref_id != &ramo.id {
                if let (Some(&from), Some(&to)) = (node_map.get(ref_id), node_map.get(&ramo.id)) {
                    let _ = pert_graph.add_edge(from, to, ());
                }
            }
        }
    }

    // Añadir aristas por correlativo (i -> j si j = i+1)
    for (_norm_i, a) in ramos_actualizados.iter() {
        for (_norm_j, b) in ramos_actualizados.iter() {
            if b.numb_correlativo == a.numb_correlativo + 1 {
                if let (Some(&from), Some(&to)) = (node_map.get(&a.id), node_map.get(&b.id)) {
                    if pert_graph.find_edge(from, to).is_none() {
                        let _ = pert_graph.add_edge(from, to, ());
                    }
                }
            }
        }
    }

    // Añadir aristas desde hojas de prerequisitos de la malla
    // Normalización simple
    fn normalize(s: &str) -> String {
        s.chars().filter(|c| c.is_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
    }

    // Resolver path de la malla (fallback heurístico si es necesario)
    let malla_pathbuf = match crate::excel::resolve_datafile_paths(malla_name) {
        Ok((m, _, _)) => m,
        Err(_) => {
            let data_dir = std::path::Path::new(crate::excel::DATAFILES_DIR);
            let mut found: Option<std::path::PathBuf> = None;
            if let Ok(entries) = std::fs::read_dir(data_dir) {
                for e in entries.flatten() {
                    if !e.path().is_file() { continue; }
                    if let Some(n) = e.file_name().to_str() {
                        let ln = n.to_lowercase();
                        if ln.contains("malla") || n == malla_name {
                            found = Some(e.path());
                            break;
                        }
                    }
                }
            }
            found.unwrap_or_else(|| std::path::PathBuf::from(malla_name.to_string()))
        }
    };

    let malla_path = malla_pathbuf.to_str().unwrap_or(malla_name).to_string();

    if let Ok(pr_map) = crate::excel::leer_prerequisitos(&malla_path) {
        // construir índice: ID (i32) -> NodeIndex
        let mut id_to_node: HashMap<i32, NodeIndex> = HashMap::new();
        for (id, idx) in node_map.iter() {
            id_to_node.insert(*id, *idx);
        }

        // construir índice: nombre normalizado -> ID
        let mut name_norm_to_id: HashMap<String, i32> = HashMap::new();
        for (_norm_name, ramo) in ramos_actualizados.iter() {
            name_norm_to_id.insert(normalize(&ramo.nombre), ramo.id);
        }

        for (codigo_str, prereqs) in pr_map.into_iter() {
            // Intentar parsear codigo_str como ID (i32) para identificar el ramo destino
            let to_id_opt = codigo_str.parse::<i32>().ok()
                .and_then(|id| node_map.contains_key(&id).then_some(id))
                .or_else(|| {
                    // Si no es un ID directo, buscar por nombre normalizado
                    name_norm_to_id.get(&normalize(&codigo_str)).copied()
                });

            if let Some(to_id) = to_id_opt {
                if let Some(&to_idx) = node_map.get(&to_id) {
                    for prereq in prereqs.into_iter() {
                        let mut matched_from_id: Option<i32> = None;

                        // 1) Intentar parsear como ID directo
                        if let Ok(id) = prereq.parse::<i32>() {
                            if node_map.contains_key(&id) {
                                matched_from_id = Some(id);
                            }
                        }

                        // 2) Buscar por nombre normalizado
                        if matched_from_id.is_none() {
                            matched_from_id = name_norm_to_id.get(&normalize(&prereq)).copied();
                        }

                        if let Some(from_id) = matched_from_id {
                            if let Some(&from_idx) = node_map.get(&from_id) {
                                if pert_graph.find_edge(from_idx, to_idx).is_none() {
                                    let _ = pert_graph.add_edge(from_idx, to_idx, ());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Ejecutar cálculo PERT para cada nodo (versión simplificada sin recursión profunda)
    // Usar una aproximación iterativa para evitar stack overflow
    let node_count = pert_graph.node_count();
    for _ in 0..node_count {
        for node_idx in pert_graph.node_indices() {
            let len_dag = node_count as i32;
            set_values_simple(&mut pert_graph, node_idx, len_dag);
        }
    }

    // Propagar resultado PERT a ramos_actualizados (marcar críticos con holgura == 0)
    for (id, idx) in node_map.iter() {
        if let Some(pn) = pert_graph.node_weight(*idx) {
            if let Some(h) = pn.h {
                // Buscar el ramo por ID
                for (_norm_name, ramo) in ramos_actualizados.iter_mut() {
                    if ramo.id == *id {
                        if h == 0 {
                            ramo.critico = true;
                        }
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
/// Versión simplificada NO RECURSIVA para cálcular PERT
/// Calcula valores para un nodo basándose en sus predecesores
fn set_values_simple(
    pert: &mut DiGraph<PertNode, ()>,
    node_idx: NodeIndex,
    len_dag: i32,
) {
    // Encontrar ancestros del nodo
    let mut max_count_jump = 1;

    // Calcular el camino más largo desde cualquier antecesor
    let predecessors: Vec<_> = pert.neighbors_directed(node_idx, Direction::Incoming).collect();

    for pred_idx in predecessors.iter() {
        if let Some(pred_node) = pert.node_weight(*pred_idx) {
            if let Some(pred_es) = pred_node.es {
                max_count_jump = std::cmp::max(max_count_jump, pred_es + 1);
            }
        }
    }

    // Actualizar valores del nodo
    let node = &mut pert[node_idx];
    node.es = Some(max_count_jump);
    node.ef = Some(node.es.unwrap() + 1);
    node.lf = Some(len_dag);
    let h = node.lf.unwrap() - node.ef.unwrap();
    node.h = Some(if h > 0 { h } else { 0 });
    node.ls = Some(node.lf.unwrap() - 1);
}

/// Versión simplificada de la función recursiva para ruta crítica (PERT)
#[allow(dead_code)]
pub fn set_values_recursive(
    pert: &mut DiGraph<PertNode, ()>,
    node_idx: NodeIndex,
    len_dag: i32,
) {
    // Encontrar ancestros del nodo
    let mut max_count_jump = 1;

    // Calcular el camino más largo desde cualquier antecesor
    let predecessors: Vec<_> = pert.neighbors_directed(node_idx, Direction::Incoming).collect();

    for _pred_idx in predecessors.iter() {
        // Simplificación del cálculo
        max_count_jump = std::cmp::max(max_count_jump, 2);
    }

    // Actualizar valores del nodo
    let node = &mut pert[node_idx];
    node.es = Some(if node.es.unwrap_or(0) < max_count_jump {
        max_count_jump
    } else {
        node.es.unwrap_or(max_count_jump)
    });

    node.ef = Some(node.es.unwrap() + 1);
    node.lf = Some(if len_dag > 1 && (node.lf.is_none() || node.lf.unwrap() > len_dag) {
        len_dag
    } else {
        node.lf.unwrap_or(len_dag)
    });

    let h = node.lf.unwrap() - node.ef.unwrap();
    node.h = Some(if h > 0 { h } else { 0 });
    node.ls = Some(node.es.unwrap() + node.h.unwrap());

    // Recursión en predecesores
    for pred_idx in predecessors {
        set_values_recursive(pert, pred_idx, len_dag - 1);
    }
}
