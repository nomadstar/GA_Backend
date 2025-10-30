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
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

    for (codigo, ramo) in ramos_actualizados.iter() {
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

    // Añadir aristas por codigo_ref
    for (codigo, ramo) in ramos_actualizados.iter() {
        if let Some(ref_code) = &ramo.codigo_ref {
            if ref_code != codigo {
                if let (Some(&from), Some(&to)) = (node_map.get(ref_code), node_map.get(codigo)) {
                    let _ = pert_graph.add_edge(from, to, ());
                }
            }
        }
    }

    // Añadir aristas por correlativo (i -> j si j = i+1)
    for (a_code, a) in ramos_actualizados.iter() {
        for (b_code, b) in ramos_actualizados.iter() {
            if b.numb_correlativo == a.numb_correlativo + 1 {
                if let (Some(&from), Some(&to)) = (node_map.get(a_code), node_map.get(b_code)) {
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
        // construir índices normalizados
        let mut code_index: HashMap<String, NodeIndex> = HashMap::new();
        for (code, idx) in node_map.iter() { code_index.insert(normalize(code), *idx); }

        let mut name_index: HashMap<String, NodeIndex> = HashMap::new();
        for s in lista_secciones.iter() {
            if let Some(&idx) = node_map.get(&s.codigo) {
                name_index.insert(normalize(&s.nombre), idx);
            }
        }

        for (codigo, prereqs) in pr_map.into_iter() {
            for prereq in prereqs.into_iter() {
                let mut matched: Option<NodeIndex> = None;

                // 1) match directo por código
                if let Some(&idx) = node_map.get(&prereq) { matched = Some(idx); }

                // 2) match por código normalizado
                if matched.is_none() {
                    let k = normalize(&prereq);
                    if let Some(&idx) = code_index.get(&k) { matched = Some(idx); }
                }

                // 3) match por nombre humano (normalizado)
                if matched.is_none() {
                    let k = normalize(&prereq);
                    if let Some(&idx) = name_index.get(&k) { matched = Some(idx); }
                }

                // 4) intentar resolver nombre -> asignatura usando asignatura_from_nombre
                if matched.is_none() {
                    if let Ok(Some(asig)) = crate::excel::asignatura_from_nombre(&malla_path, &prereq) {
                        if let Some(&idx) = node_map.get(&asig) { matched = Some(idx); }
                        else if let Some(&idx) = code_index.get(&normalize(&asig)) { matched = Some(idx); }
                    }
                }

                if let Some(from_idx) = matched {
                    if let Some(&to_idx) = node_map.get(&codigo) {
                        if pert_graph.find_edge(from_idx, to_idx).is_none() {
                            let _ = pert_graph.add_edge(from_idx, to_idx, ());
                        }
                    }
                }
            }
        }
    }

    // Ejecutar cálculo PERT para cada nodo
    for node_idx in pert_graph.node_indices() {
        let len_dag = pert_graph.node_count() as i32;
        crate::algorithm::pert::set_values_recursive(&mut pert_graph, node_idx, len_dag);
    }

    // Propagar resultado PERT a ramos_actualizados (marcar críticos con holgura == 0)
    for (codigo, idx) in node_map.iter() {
        if let Some(pn) = pert_graph.node_weight(*idx) {
            if let Some(h) = pn.h {
                if let Some(r) = ramos_actualizados.get_mut(codigo) {
                    if h == 0 {
                        r.critico = true;
                    }
                }
            }
        }
    }

    Ok(())
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
