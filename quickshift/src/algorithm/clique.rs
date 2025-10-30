use std::collections::HashMap;
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::algorithm::conflict::horarios_tienen_conflicto;

 
pub fn find_max_weight_clique(
    graph: &UnGraph<usize, ()>,
    priorities: &HashMap<NodeIndex, i32>,
) -> Vec<NodeIndex> {
    let nodes: Vec<_> = graph.node_indices().collect();
    let mut sorted_nodes = nodes.clone();
    sorted_nodes.sort_by(|&a, &b| {
        priorities.get(&b).unwrap_or(&0).cmp(priorities.get(&a).unwrap_or(&0))
    });

    let mut current_clique = Vec::new();
    if let Some(&first_node) = sorted_nodes.first() {
        current_clique.push(first_node);
    }

    for &node in sorted_nodes.iter().skip(1) {
        let mut compatible = true;
        for &clique_node in &current_clique {
            if !graph.contains_edge(node, clique_node) {
                compatible = false;
                break;
            }
        }
        if compatible {
            current_clique.push(node);
            if current_clique.len() >= 6 { break; }
        }
    }

    current_clique
}

pub fn get_clique_max_pond(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    println!("=== Generador de Horarios ===");
    println!("Ramos disponibles:\n");
    for (i, (codigo, ramo)) in ramos_disponibles.iter().enumerate() {
        println!("{}.- {} || {}", i, ramo.nombre, codigo);
    }

    let mut priority_ramo: HashMap<String, i32> = HashMap::new();
    let mut priority_sec: HashMap<String, i32> = HashMap::new();

    priority_ramo.insert("Algoritmos y Programación".to_string(), 90);
    priority_ramo.insert("Bases de Datos".to_string(), 85);
    priority_sec.insert("CIT3313-SEC1".to_string(), 95);

    let mut graph = UnGraph::<usize, ()>::new_undirected();
    let mut node_indices = Vec::new();
    let mut priorities = HashMap::new();

    for (idx, seccion) in lista_secciones.iter().enumerate() {
        // Buscar por nombre normalizado (para NO-ELECTIVOS)
        let nombre_norm = crate::excel::normalize_name(&seccion.nombre);
        let ramo = if let Some(r) = ramos_disponibles.get(&nombre_norm) {
            Some(r)
        } else if nombre_norm == "electivo profesional" {
            // CASO ESPECIAL: Para electivos, la clave es diferente
            // Intentar buscar por la clave patrón usado en leer_malla_con_porcentajes
            // Pero como no tenemos el ID aquí, solo buscar el primer electivo disponible
            ramos_disponibles.iter()
                .find(|(k, _)| k.starts_with("electivo_profesional_"))
                .map(|(_, r)| r)
        } else {
            None
        };

        let ramo = match ramo {
            Some(r) => r,
            None => {
                eprintln!("WARN: No se encontró ramo con nombre normalizado '{}' (original: '{}', código: '{}')", nombre_norm, seccion.nombre, seccion.codigo);
                continue;
            }
        };
        
        let cc = if ramo.critico { 10 } else { 0 };
        let uu = 10 - ramo.holgura;
        let mut kk = 60 - ramo.numb_correlativo;

        if let Some(&prio) = priority_ramo.get(&seccion.nombre) {
            kk = prio + 53;
        }

        let mut ss = seccion.seccion.parse::<i32>().unwrap_or(0);
        if let Some(&prio) = priority_sec.get(&seccion.codigo) {
            ss = prio + 20;
        }

        let prioridad = cc * 10000 + uu * 1000 + kk * 100 + ss;

        let node_idx = graph.add_node(idx);
        node_indices.push(node_idx);
        priorities.insert(node_idx, prioridad);
    }

    for i in 0..node_indices.len() {
        for j in (i + 1)..node_indices.len() {
            let sec_i = &lista_secciones[graph[node_indices[i]]];
            let sec_j = &lista_secciones[graph[node_indices[j]]];

            if sec_i.codigo_box != sec_j.codigo_box &&
               sec_i.codigo[..std::cmp::min(7, sec_i.codigo.len())] != 
               sec_j.codigo[..std::cmp::min(7, sec_j.codigo.len())] {

                if !horarios_tienen_conflicto(&sec_i.horario, &sec_j.horario) {
                    graph.add_edge(node_indices[i], node_indices[j], ());
                }
            }
        }
    }

    println!("\n=== Soluciones Recomendadas ===");

    let mut prev_solutions = Vec::new();
    let mut graph_copy = graph.clone();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    for _solution_num in 1..=5 {
        let max_clique = find_max_weight_clique(&graph_copy, &priorities);
        if max_clique.len() <= 2 { break; }

        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();

        arr_aux_delete.sort_by_key(|&(_, prio)| prio);

        while arr_aux_delete.len() > 6 { arr_aux_delete.remove(0); }

        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        if prev_solutions.contains(&solution_key) {
            if !arr_aux_delete.is_empty() { graph_copy.remove_node(arr_aux_delete[0].0); }
            continue;
        }

        let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
        let mut total_score_i64: i64 = 0;

        for &(node_idx, prioridad) in &arr_aux_delete {
            let seccion_idx = graph_copy[node_idx];
            let seccion = lista_secciones[seccion_idx].clone();
            println!("{} || {} - Sección: {} | Horario -> {:?} || {}",
                &seccion.codigo[..std::cmp::min(7, seccion.codigo.len())],
                seccion.nombre,
                seccion.seccion,
                seccion.horario,
                prioridad
            );

            solution_entries.push((seccion, prioridad));
            total_score_i64 += prioridad as i64;
        }

        solutions.push((solution_entries, total_score_i64));
        prev_solutions.push(solution_key);

        if !arr_aux_delete.is_empty() { graph_copy.remove_node(arr_aux_delete[0].0); }
    }

    solutions
}

pub fn get_clique_max_pond_with_prefs(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &crate::api_json::InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let mut filtered: Vec<Seccion> = Vec::new();
    for s in lista_secciones.iter() {
        let mut is_taken = false;
        for rp in params.ramos_pasados.iter() {
            if rp == &s.codigo_box || s.codigo.starts_with(rp) { is_taken = true; break; }
        }
        if !is_taken { filtered.push(s.clone()); }
    }

    let mut priority_ramo: HashMap<String, i32> = HashMap::new();
    let priority_sec: HashMap<String, i32> = HashMap::new();

    for rp in params.ramos_prioritarios.iter() {
        priority_ramo.insert(rp.clone(), 5000);
    }

    let mut graph = UnGraph::<usize, ()>::new_undirected();
    let mut node_indices = Vec::new();
    let mut priorities = HashMap::new();

    for (idx, seccion) in filtered.iter().enumerate() {
        // Buscar por nombre normalizado (para NO-ELECTIVOS)
        let nombre_norm = crate::excel::normalize_name(&seccion.nombre);
        
        let ramo = if let Some(r) = ramos_disponibles.get(&nombre_norm) {
            Some(r)
        } else if nombre_norm == "electivo profesional" {
            // CASO ESPECIAL: Para electivos, buscar por clave patrón
            ramos_disponibles.iter()
                .find(|(k, _)| k.starts_with("electivo_profesional_"))
                .map(|(_, r)| r)
        } else {
            None
        };

        if ramo.is_none() {
            continue;
        }

        let ramo = ramo.unwrap();
        let cc = if ramo.critico { 10 } else { 0 };
        let uu = 10 - ramo.holgura;
        let mut kk = 60 - ramo.numb_correlativo;

        if let Some(&prio) = priority_ramo.get(&seccion.nombre) { kk = prio + 53; }
        if let Some(&prio) = priority_ramo.get(&nombre_norm) { kk = prio + 53; }

        let mut ss = seccion.seccion.parse::<i32>().unwrap_or(0);
        if let Some(&prio) = priority_sec.get(&seccion.codigo) { ss = prio + 20; }

        let mut horario_boost = 0;
        for pref in params.horarios_preferidos.iter() {
            for h in seccion.horario.iter() {
                if h.contains(pref) || pref.contains(h) {
                    horario_boost = 2000;
                    break;
                }
            }
            if horario_boost > 0 { break; }
        }

        let prioridad = cc * 10000 + uu * 1000 + kk * 100 + ss + horario_boost;
        let node_idx = graph.add_node(idx);
        node_indices.push(node_idx);
        priorities.insert(node_idx, prioridad);
    }

    for i in 0..node_indices.len() {
        for j in (i + 1)..node_indices.len() {
            let sec_i = &filtered[graph[node_indices[i]]];
            let sec_j = &filtered[graph[node_indices[j]]];

            if sec_i.codigo_box != sec_j.codigo_box &&
               sec_i.codigo[..std::cmp::min(7, sec_i.codigo.len())] != 
               sec_j.codigo[..std::cmp::min(7, sec_j.codigo.len())] {

                if !horarios_tienen_conflicto(&sec_i.horario, &sec_j.horario) {
                    graph.add_edge(node_indices[i], node_indices[j], ());
                }
            }
        }
    }

    let mut prev_solutions = Vec::new();
    let mut graph_copy = graph.clone();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    for _solution_num in 1..=5 {
        let max_clique = find_max_weight_clique(&graph_copy, &priorities);
        if max_clique.len() <= 2 { break; }

        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();

        arr_aux_delete.sort_by_key(|&(_, prio)| prio);
        while arr_aux_delete.len() > 6 { arr_aux_delete.remove(0); }

        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        if prev_solutions.contains(&solution_key) {
            if !arr_aux_delete.is_empty() { graph_copy.remove_node(arr_aux_delete[0].0); }
            continue;
        }

        let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
        let mut total_score_i64: i64 = 0;

        for &(node_idx, prioridad) in &arr_aux_delete {
            let seccion_idx = graph_copy[node_idx];
            let seccion = filtered[seccion_idx].clone();
            solution_entries.push((seccion, prioridad));
            total_score_i64 += prioridad as i64;
        }

        solutions.push((solution_entries, total_score_i64));
        prev_solutions.push(solution_key);

        if !arr_aux_delete.is_empty() { graph_copy.remove_node(arr_aux_delete[0].0); }
    }

    solutions
}

/// Public wrapper kept in this module so the implementation and its API
/// live together. Delegates to `get_clique_max_pond_with_prefs` which
/// already applies `ramos_pasados`, `ramos_prioritarios` and
/// `horarios_preferidos` from `InputParams`.
pub fn get_clique_with_user_prefs(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &crate::api_json::InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    get_clique_max_pond_with_prefs(lista_secciones, ramos_disponibles, params)
}
