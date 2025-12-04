/// clique.rs - Planificador minimalista: PERT + Cliques + Restricciones integradas
use std::collections::{HashMap, HashSet};
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::excel::normalize_name;
use crate::api_json::InputParams;

// Extrae la clave base de un curso (quita sufijos tipo 'laboratorio', 'taller', 'práctica')
fn base_course_key(nombre: &str) -> String {
    let mut s = nombre.to_lowercase();
    // remover tokens comunes
    for t in &["laboratorio", "laboratorios", "lab", "taller", "talleres", "practica", "práctica", "practicas", "prácticas"] {
        s = s.replace(t, "");
    }
    // quitar caracteres no alfanuméricos y normalizar
    normalize_name(&s)
}

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
    // Implementación directa y concisa de "cliques reales" (greedy multi-seed).
    eprintln!("🧠 [clique] {} secciones, {} ramos", lista_secciones.len(), ramos_disponibles.len());

    // --- Filtrado inicial (semestre y ramos pasados) ---
    let mut max_sem = 0;
    for code in &params.ramos_pasados {
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == *code) {
            if let Some(s) = r.semestre { max_sem = max_sem.max(s); }
        }
    }
    let max_sem = max_sem + 2;
    let passed: HashSet<_> = params.ramos_pasados.iter().cloned().collect();

    let filtered: Vec<Seccion> = lista_secciones.iter().filter(|s| {
        if passed.contains(&s.codigo_box) { return false; }
        // aceptar si el ramo existe en la malla (por nombre normalizado) o si no tiene semestre fuera del horizonte
        let ramo_ok = ramos_disponibles.values().find(|r| r.codigo == s.codigo)
            .map_or(true, |r| r.semestre.map_or(true, |sem| sem <= max_sem));
        ramo_ok
    }).cloned().collect();

    eprintln!("   Filtrado: {} secciones", filtered.len());

    // --- Construir matriz de compatibilidad (adjacency) ---
    let n = filtered.len();
    let mut adj = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i+1)..n {
            let s1 = &filtered[i];
            let s2 = &filtered[j];
            let code_a = &s1.codigo[..std::cmp::min(7, s1.codigo.len())];
            let code_b = &s2.codigo[..std::cmp::min(7, s2.codigo.len())];
            if s1.codigo_box != s2.codigo_box && code_a != code_b && !sections_conflict(s1, s2) {
                adj[i][j] = true; adj[j][i] = true;
            }
        }
    }

    // --- Prioridades por sección (resolver RamoDisponible por código o nombre normalizado) ---
    let mut pri: Vec<i64> = Vec::with_capacity(n);
    for s in filtered.iter() {
        let candidate = ramos_disponibles.values().find(|r| {
            if !r.codigo.is_empty() && !s.codigo.is_empty() {
                if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
            }
            normalize_name(&r.nombre) == normalize_name(&s.nombre)
        });
        let p = candidate.map(|r| compute_priority(r, s)).unwrap_or(0);
        pri.push(p);
    }

    // --- Greedy multi-seed to build real cliques ---
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| -(pri[i] as i64));

    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let max_seeds = std::cmp::min(30, n);
    for &seed_idx in order.iter().take(max_seeds) {
        let mut clique: Vec<usize> = vec![seed_idx];
        for &cand in order.iter() {
            if cand == seed_idx { continue; }
            // candidate must be connected to ALL nodes already in clique
            if clique.iter().all(|&u| adj[u][cand]) {
                // Además: si cand y algún u pertenecen a la misma materia base,
                // exigir que pertenezcan a la misma `seccion` (emparejar laboratorios/talleres)
                let mut conflict = false;
                let cand_key = base_course_key(&filtered[cand].nombre);
                let cand_seccion = filtered[cand].seccion.clone();
                for &u in clique.iter() {
                    let u_key = base_course_key(&filtered[u].nombre);
                    let u_seccion = &filtered[u].seccion;
                    if !cand_key.is_empty() && cand_key == u_key {
                        if u_seccion != &cand_seccion {
                            conflict = true;
                            break;
                        }
                    }
                }
                if !conflict {
                    clique.push(cand);
                }
            }
            if clique.len() >= 6 { break; }
        }

        // mapear clique a solución (Seccion + score) usando matching robusto
        let mut sol: Vec<(Seccion, i32)> = Vec::new();
        let mut total: i64 = 0;
        for &ix in clique.iter() {
            let s = filtered[ix].clone();
            if let Some(r) = ramos_disponibles.values().find(|r| {
                if !r.codigo.is_empty() && !s.codigo.is_empty() {
                    if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
                }
                normalize_name(&r.nombre) == normalize_name(&s.nombre)
            }) {
                let score = compute_priority(r, &s);
                sol.push((s, score as i32));
                total += score;
            }
        }
        if !sol.is_empty() {
            solutions.push((sol, total));
        }
    }

    // ordenar y truncar
    solutions.sort_by(|a, b| b.1.cmp(&a.1));
    solutions.truncate(10);
    eprintln!("✅ [clique] {} soluciones (real cliques greedy)", solutions.len());
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
