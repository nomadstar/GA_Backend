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
    // Fórmula correcta del RutaCritica.py:
    // priority = CC + UU + KK + SS (concatenación como string, luego a int)
    // CC: "10" if critico else "00"
    // UU: f"{10-holgura:02d}"
    // KK: f"{60-numb_correlativo:02d}"
    // SS: f"{seccion_number:02d}"
    
    let cc_str = if ramo.critico { "10" } else { "00" };
    
    let holgura_int = (ramo.holgura as i32).max(0).min(10);
    let uu_val = 10 - holgura_int;
    let uu_str = format!("{:02}", uu_val);
    
    let numb_corr_int = ramo.numb_correlativo.max(0);
    let kk_val = 60 - numb_corr_int;
    let kk_str = format!("{:02}", kk_val.max(0).min(60));
    
    // SS: extraer número de seccion
    let ss_str = if let Ok(sec_num) = sec.seccion.parse::<i32>() {
        format!("{:02}", sec_num.max(0).min(99))
    } else {
        "00".to_string()
    };
    
    let priority_str = format!("{}{}{}{}", cc_str, uu_str, kk_str, ss_str);
    priority_str.parse::<i64>().unwrap_or(0)
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

    // --- Greedy multi-seed to build real cliques with max 6 courses ---
    // Itera múltiples veces removiendo el mejor nodo cada vez para obtener soluciones diversas
    let mut all_solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
    let mut remaining_indices: HashSet<usize> = (0..n).collect();
    let max_iterations = 10;  // Buscar hasta 10 soluciones
    
    for _iteration in 0..max_iterations {
        if remaining_indices.is_empty() {
            break;
        }
        
        // Ordenar por prioridad dentro de índices restantes
        let mut candidates: Vec<usize> = remaining_indices.iter().copied().collect();
        candidates.sort_by_key(|&i| -(pri[i] as i64));
        
        if candidates.is_empty() {
            break;
        }
        
        let seed_idx = candidates[0];
        let mut clique: Vec<usize> = vec![seed_idx];
        
        // Greedy: agregar candidatos conectados a todos en la clique, max 6
        for &cand in candidates.iter().skip(1) {
            if clique.len() >= 6 {
                break;
            }
            if !remaining_indices.contains(&cand) {
                continue;
            }
            
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
        }

        // mapear clique a solución (Seccion + score)
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
                sol.push((s.clone(), score as i32));
                total += score;
            }
        }
        
        if !sol.is_empty() {
            all_solutions.push((sol, total));
            
            // Remover el mejor nodo de esta clique del conjunto restante para la siguiente iteración
            if let Some(&best_node) = clique.iter().max_by_key(|&&i| pri[i]) {
                remaining_indices.remove(&best_node);
            }
        } else {
            // Si no hay solución válida, remover el seed
            remaining_indices.remove(&seed_idx);
        }
    }

    // ordenar por score y truncar a 10 soluciones
    all_solutions.sort_by(|a, b| b.1.cmp(&a.1));
    all_solutions.truncate(10);
    eprintln!("✅ [clique] {} soluciones (max_weight_clique, max 6 ramos, iteraciones)", all_solutions.len());
    all_solutions
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
