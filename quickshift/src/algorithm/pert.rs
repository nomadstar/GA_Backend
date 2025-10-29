use petgraph::graph::{NodeIndex, DiGraph};
use petgraph::Direction;
use crate::models::PertNode;

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
