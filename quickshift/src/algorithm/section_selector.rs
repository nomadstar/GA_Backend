use crate::models::Seccion;
use crate::algorithm::conflict::horarios_tienen_conflicto;

/// Dado un conjunto de candidatos por ramo (Vec por ramo -> Vec<Seccion>),
/// intenta seleccionar exactamente una `Seccion` por ramo sin solapamientos.
///
/// Estrategia: backtracking con ordenación por número de candidatos (menos
/// ramas primero) y orden determinista de secciones dentro de cada grupo.
/// Devuelve la primera asignación válida encontrada (determinista).
pub fn select_non_conflicting_sections(candidate_groups: &Vec<Vec<Seccion>>) -> Option<Vec<Seccion>> {
    if candidate_groups.is_empty() { return Some(vec![]); }

    // Clonar y ordenar determinísticamente cada grupo de candidatos
    let mut groups: Vec<Vec<Seccion>> = candidate_groups.iter().map(|g| {
        let mut v = g.clone();
        v.sort_by(|a, b| {
            let ka = format!("{}::{}::{}", a.codigo_box, a.codigo, a.seccion);
            let kb = format!("{}::{}::{}", b.codigo_box, b.codigo, b.seccion);
            ka.cmp(&kb)
        });
        v
    }).collect();

    // Construir orden de iteración: ramas con menos candidatos primero
    let mut order: Vec<usize> = (0..groups.len()).collect();
    order.sort_by_key(|&i| (groups[i].len(), i));

    let mut assignment: Vec<Option<Seccion>> = vec![None; groups.len()];
    let mut chosen: Vec<Seccion> = Vec::new();

    fn backtrack(pos: usize, order: &Vec<usize>, groups: &Vec<Vec<Seccion>>, assignment: &mut Vec<Option<Seccion>>, chosen: &mut Vec<Seccion>) -> bool {
        if pos == order.len() { return true; }
        let idx = order[pos];
        for sect in groups[idx].iter() {
            if chosen.iter().any(|c| horarios_tienen_conflicto(&c.horario, &sect.horario)) { continue; }
            chosen.push(sect.clone());
            assignment[idx] = Some(sect.clone());
            if backtrack(pos + 1, order, groups, assignment, chosen) { return true; }
            chosen.pop();
            assignment[idx] = None;
        }
        false
    }

    if backtrack(0, &order, &groups, &mut assignment, &mut chosen) {
        let mut out: Vec<Seccion> = Vec::new();
        for a in assignment.into_iter() {
            if let Some(s) = a { out.push(s); } else { return None; }
        }
        Some(out)
    } else {
        None
    }
}
