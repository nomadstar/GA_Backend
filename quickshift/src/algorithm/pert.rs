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

    // Construir conjunto de códigos presentes en `lista_secciones` para
    // excluir ramos que no tienen secciones (filtrado de filas vacías OA).
    use std::collections::HashSet;
    let present_codes: HashSet<String> = lista_secciones.iter()
        .map(|s| s.codigo.trim().to_ascii_uppercase())
        .collect();

    for (code_key, ramo) in ramos_actualizados.iter() {
        // `code_key` corresponde a la clave usada en `ramos_actualizados` y
        // normalmente coincide con `Seccion.codigo`. Si no está presente en
        // `lista_secciones`, saltamos el ramo.
        if !present_codes.contains(&code_key.trim().to_ascii_uppercase()) {
            continue;
        }

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
    // Agrupamos por `numb_correlativo` y conectamos elementos consecutivos
    {
        use std::collections::BTreeMap;
        let mut by_correl: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
        for (_k, r) in ramos_actualizados.iter() {
            by_correl.entry(r.numb_correlativo).or_default().push(r.id);
        }
        for (_cor, mut ids) in by_correl.into_iter() {
            if ids.len() <= 1 { continue; }
            ids.sort_unstable();
            for win in ids.windows(2) {
                let a = win[0];
                let b = win[1];
                if let (Some(&from), Some(&to)) = (node_map.get(&a), node_map.get(&b)) {
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

    // Intentar obtener prerequisitos desde el caché en memoria; si falla,
    // el error se propaga y no añadimos aristas por prereqs.
    if let Ok(pr_map_arc) = crate::excel::get_prereqs_cached(&malla_path) {
        let pr_map: &std::collections::HashMap<String, Vec<String>> = &*pr_map_arc;
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
        for (codigo_str, prereqs) in pr_map.iter() {
            // Intentar parsear codigo_str como ID (i32) para identificar el ramo destino
            let to_id_opt = codigo_str.parse::<i32>().ok()
                .and_then(|id| node_map.contains_key(&id).then_some(id))
                .or_else(|| {
                    // Si no es un ID directo, buscar por nombre normalizado
                    name_norm_to_id.get(&normalize(&codigo_str)).copied()
                });

            if let Some(to_id) = to_id_opt {
                if let Some(&to_idx) = node_map.get(&to_id) {
                    for prereq in prereqs.iter() {
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

    // Ejecutar cálculo PERT usando orden topológico (forward/backward) -> O(N + E)
    use petgraph::algo::toposort;
    let topo = match toposort(&pert_graph, None) {
        Ok(order) => order,
        Err(_) => {
            // En caso de ciclo, hacer fallback limitado (evitamos bucles infinitos)
            eprintln!("WARNING: PERT graph contains a cycle; using limited iterative fallback");
            let node_count = pert_graph.node_count();
            for _ in 0..3 {
                for node_idx in pert_graph.node_indices() {
                    let len_dag = node_count as i32;
                    set_values_simple(&mut pert_graph, node_idx, len_dag);
                }
            }
            // Propagar resultado PERT (igual que abajo) y volver
            for (id, idx) in node_map.iter() {
                if let Some(pn) = pert_graph.node_weight(*idx) {
                    if let Some(h) = pn.h {
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
            return Ok(());
        }
    };

    // Forward pass: calcular ES / EF (usar DP sobre el orden topológico)
    // Inicializar ES a 1
    for &node_idx in topo.iter() {
        if let Some(node) = pert_graph.node_weight_mut(node_idx) {
            node.es = Some(1);
            node.ef = Some(2); // es + dur (dur=1)
        }
    }
    // Propagar longitudes máximas a lo largo del DAG: for each u in topo, for each v in out(u): es[v] = max(es[v], ef[u])
    for &u in topo.iter() {
        let u_ef = pert_graph.node_weight(u).and_then(|n| n.ef).unwrap_or(1);
        // recoger vecinos salientes primero para evitar préstamos simultáneos
        let outs: Vec<_> = pert_graph.neighbors_directed(u, Direction::Outgoing).collect();
        for v in outs {
            if let Some(vnode) = pert_graph.node_weight_mut(v) {
                if vnode.es.unwrap_or(1) < u_ef {
                    vnode.es = Some(u_ef);
                    vnode.ef = Some(u_ef + 1);
                }
            }
        }
    }

    // Backward pass: calcular LF / LS / h (usar reverse topo)
    let max_ef = topo.iter().filter_map(|&n| pert_graph.node_weight(n).and_then(|nn| nn.ef)).max().unwrap_or(1);
    for &node_idx in topo.iter().rev() {
        let mut lf = max_ef;
        let mut has_succ = false;
        for succ in pert_graph.neighbors_directed(node_idx, Direction::Outgoing) {
            if let Some(succ_node) = pert_graph.node_weight(succ) {
                if let Some(succ_ls) = succ_node.ls {
                    lf = std::cmp::min(lf, succ_ls);
                } else if let Some(succ_es) = succ_node.es {
                    lf = std::cmp::min(lf, succ_es + 1);
                }
                has_succ = true;
            }
        }
        if !has_succ {
            lf = max_ef;
        }
        if let Some(node) = pert_graph.node_weight_mut(node_idx) {
            node.lf = Some(lf);
            node.ls = Some(lf - 1);
            let h = node.lf.unwrap() - node.ef.unwrap_or(node.lf.unwrap());
            node.h = Some(if h > 0 { h } else { 0 });
        }
    }

    // Propagar resultado PERT a ramos_actualizados (marcar críticos con holgura == 0)
    for (id, idx) in node_map.iter() {
        if let Some(pn) = pert_graph.node_weight(*idx) {
            if let Some(h) = pn.h {
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
