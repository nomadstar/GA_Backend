/// clique.rs - Planificador minimalista: PERT + Cliques + Restricciones integradas
use std::collections::{HashMap, HashSet};
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::api_json::InputParams;

fn compute_priority(ramo: &RamoDisponible, sec: &Seccion) -> i64 {
    let cc = if ramo.critico { 10 } else { 0 };
    let uu = std::cmp::min(9, (10 - ramo.holgura as i64).max(0));
    let kk = (60 - ramo.numb_correlativo as i64).max(0);
    let ss = sec.seccion.parse::<i64>().unwrap_or(0);
    cc * 1_000_000 + uu * 100_000 + kk * 100 + ss
}

fn sections_conflict(s1: &Seccion, s2: &Seccion) -> bool {
    s1.horario.iter().any(|h1| s2.horario.iter().any(|h2| h1 == h2))
}

pub fn get_clique_max_pond_with_prefs(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    eprintln!("🧠 [clique] {} secciones, {} ramos", lista_secciones.len(), ramos_disponibles.len());

    let mut max_sem = 0;
    for code in &params.ramos_pasados {
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == *code) {
            if let Some(s) = r.semestre { max_sem = max_sem.max(s); }
        }
    }
    let max_sem = max_sem + 2;
    let passed: HashSet<_> = params.ramos_pasados.iter().cloned().collect();

    let filtered: Vec<_> = lista_secciones.iter().filter(|s| {
        !passed.contains(&s.codigo_box) && 
        ramos_disponibles.values().find(|r| r.codigo == s.codigo)
            .map_or(true, |r| r.semestre.map_or(true, |sem| sem <= max_sem))
    }).cloned().collect();

    eprintln!("   Filtrado: {} secciones", filtered.len());

    let mut graph = UnGraph::<Seccion, ()>::new_undirected();
    let nodes: Vec<_> = filtered.iter().map(|s| graph.add_node(s.clone())).collect();

    for i in 0..nodes.len() {
        for j in (i+1)..nodes.len() {
            let s1 = graph.node_weight(nodes[i]).unwrap();
            let s2 = graph.node_weight(nodes[j]).unwrap();
            let code_a = &s1.codigo[0..std::cmp::min(7, s1.codigo.len())];
            let code_b = &s2.codigo[0..std::cmp::min(7, s2.codigo.len())];
            if s1.codigo_box != s2.codigo_box && code_a != code_b && !sections_conflict(s1, s2) {
                graph.add_edge(nodes[i], nodes[j], ());
            }
        }
    }

    let mut solutions = Vec::new();
    let mut removed = HashSet::new();

    for _ in 0..10 {
        let best = graph.node_indices().filter(|n| !removed.contains(n))
            .max_by_key(|n| {
                let sec = graph.node_weight(*n).unwrap();
                ramos_disponibles.values().find(|r| r.codigo == sec.codigo)
                    .map(|r| compute_priority(r, sec)).unwrap_or(0)
            });

        let Some(start) = best else { break; };
        let mut clique = vec![start];

        for _ in 1..6 {
            let cands: Vec<_> = graph.neighbors(clique[clique.len()-1])
                .filter(|n| !removed.contains(n) && !clique.contains(n)).collect();
            if cands.is_empty() { break; }
            let next = cands.into_iter().max_by_key(|n| {
                let sec = graph.node_weight(*n).unwrap();
                ramos_disponibles.values().find(|r| r.codigo == sec.codigo)
                    .map(|r| compute_priority(r, sec)).unwrap_or(0)
            }).unwrap();
            clique.push(next);
        }

        let mut sol = Vec::new();
        let mut total = 0i64;
        for &idx in &clique {
            let sec = graph.node_weight(idx).unwrap().clone();
            if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == sec.codigo) {
                let score = compute_priority(r, &sec);
                sol.push((sec, score as i32));
                total += score;
            }
        }

        if !sol.is_empty() { solutions.push((sol, total)); }
        removed.insert(start);
    }

    solutions.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("✅ [clique] {} soluciones", solutions.len());
    solutions.truncate(10);
    solutions
}

/// Wrapper público
pub fn get_clique_with_user_prefs(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    get_clique_max_pond_with_prefs(lista_secciones, ramos_disponibles, params)
}

pub fn get_clique_dependencies_only(
    lista_secciones: &[Seccion],
    _ramos_disponibles: &HashMap<String, RamoDisponible>,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let mut graph = UnGraph::<Seccion, ()>::new_undirected();
    let nodes: Vec<_> = lista_secciones.iter().map(|s| graph.add_node(s.clone())).collect();

    for i in 0..nodes.len() {
        for j in (i+1)..nodes.len() {
            if graph.node_weight(nodes[i]).unwrap().codigo_box != 
               graph.node_weight(nodes[j]).unwrap().codigo_box {
                graph.add_edge(nodes[i], nodes[j], ());
            }
        }
    }

    let sol: Vec<_> = nodes.iter().take(6).map(|&n| 
        (graph.node_weight(n).unwrap().clone(), 50)
    ).collect();
    
    if sol.is_empty() { vec![] } else { vec![(sol, 300)] }
}
