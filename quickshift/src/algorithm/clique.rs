use std::collections::HashMap;
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::algorithm::conflict::{horarios_tienen_conflicto, horarios_violate_min_gap};
use std::time::Instant;

/// Construir un índice inverso: PA2025-1 código → clave de HashMap (para electivos)
/// Permite buscar un electivo por su código de PA2025-1
fn build_code_to_key_index(ramos_disponibles: &HashMap<String, RamoDisponible>) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for (key, ramo) in ramos_disponibles.iter() {
        if ramo.electivo {
            // Mapear código de PA2025-1 → clave del HashMap
            index.insert(ramo.codigo.clone(), key.clone());
        }
    }
    index
}

/// Construir índice PA2025-1 código → nombre normalizado para TODOS los ramos
/// (no solo electivos). Esto permite resolver ramos_prioritarios.
fn build_code_to_name_index(ramos_disponibles: &HashMap<String, RamoDisponible>) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for (key, ramo) in ramos_disponibles.iter() {
        // Mapear código de PA2025-1 → nombre normalizado (clave del HashMap)
        index.insert(ramo.codigo.clone(), key.clone());
    }
    index
}

pub fn find_max_weight_clique(
    graph: &UnGraph<usize, ()>,
    priorities: &HashMap<NodeIndex, i32>,
) -> Vec<NodeIndex> {
    // Búsqueda heurística multi-seed: intentamos arrancar desde varias semillas
    // (nodos de mayor prioridad) y elegimos la clique con mayor suma de prioridades.
    let nodes: Vec<_> = graph.node_indices().collect();
    let mut sorted_nodes = nodes.clone();
    sorted_nodes.sort_by(|&a, &b| {
        priorities.get(&b).unwrap_or(&0).cmp(priorities.get(&a).unwrap_or(&0))
    });

    // número de semillas a probar (tuneable). Elegir un número razonable.
    let max_seeds = std::cmp::min(50, sorted_nodes.len());

    let mut best_clique: Vec<NodeIndex> = Vec::new();
    let mut best_score: i64 = std::i64::MIN;

    // Helper para calcular score de una clique
    let clique_score = |clique: &Vec<NodeIndex>| -> i64 {
        clique.iter().map(|n| *priorities.get(n).unwrap_or(&0) as i64).sum()
    };

    // Intento 1: greedy normal (empieza por el mayor)
    {
        let mut clique: Vec<NodeIndex> = Vec::new();
        if let Some(&first_node) = sorted_nodes.first() {
            clique.push(first_node);
            for &node in sorted_nodes.iter().skip(1) {
                let mut compatible = true;
                for &clique_node in &clique {
                    if !graph.contains_edge(node, clique_node) {
                        compatible = false; break;
                    }
                }
                if compatible { clique.push(node); }
            }
        }
        let score = clique_score(&clique);
        if score > best_score {
            best_score = score; best_clique = clique;
        }
    }

    // Intentos adicionales: iniciar en cada una de las top seeds
    for &seed in sorted_nodes.iter().take(max_seeds) {
        let mut clique: Vec<NodeIndex> = Vec::new();
        clique.push(seed);
        for &node in sorted_nodes.iter() {
            if node == seed { continue; }
            let mut compatible = true;
            for &clique_node in &clique {
                if !graph.contains_edge(node, clique_node) {
                    compatible = false; break;
                }
            }
            if compatible { clique.push(node); }
        }
        let score = clique_score(&clique);
        if score > best_score {
            best_score = score; best_clique = clique;
        }
    }

    best_clique
}

/// Variante de la heurística que aplica un `seed` para tie-breaking
/// entre nodos con similar prioridad. Esto permite realizar múltiples
/// reinicios con diferentes ordenamientos y aumentar la probabilidad
/// de encontrar cliques de mayor peso.
fn find_max_weight_clique_with_seed(
    graph: &UnGraph<usize, ()>,
    priorities: &HashMap<NodeIndex, i32>,
    seed: usize,
) -> Vec<NodeIndex> {
    let nodes: Vec<_> = graph.node_indices().collect();
    let mut sorted_nodes = nodes.clone();
    sorted_nodes.sort_by(|&a, &b| {
        let pa = *priorities.get(&a).unwrap_or(&0);
        let pb = *priorities.get(&b).unwrap_or(&0);
        // Orden principal por prioridad descendente
        match pb.cmp(&pa) {
            std::cmp::Ordering::Equal => {
                // Tie-breaker: usar seed combinado con el index para variar el orden
                let ta = (a.index() ^ seed) as isize;
                let tb = (b.index() ^ seed) as isize;
                tb.cmp(&ta)
            }
            other => other,
        }
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
    eprintln!("DEBUG get_clique_max_pond: {} secciones, {} ramos disponibles", 
              lista_secciones.len(), ramos_disponibles.len());
    println!("=== Generador de Horarios ===");
    println!("Ramos disponibles:\n");
    for (i, (codigo, ramo)) in ramos_disponibles.iter().enumerate() {
        println!("{}.- {} || {}", i, ramo.nombre, codigo);
    }

    // Construir índice inverso PA2025-1 código → clave del HashMap
    let code_to_key = build_code_to_key_index(ramos_disponibles);

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
            // CASO ESPECIAL: Para electivos, buscar por el código de PA2025-1
            // El código en la sección es el código de PA2025-1 del electivo asignado
            if let Some(key) = code_to_key.get(&seccion.codigo) {
                ramos_disponibles.get(key)
            } else {
                eprintln!("WARN: Electivo con código '{}' no encontrado en code_to_key", seccion.codigo);
                None
            }
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
        // Buscar la mejor clique probando varias semillas de tie-breaking
        let mut best_clique: Vec<NodeIndex> = Vec::new();
        let mut best_clique_score: i64 = std::i64::MIN;
        let seed_attempts = 8usize;
        for seed in 0..seed_attempts {
            let c = find_max_weight_clique_with_seed(&graph_copy, &priorities, seed);
            if c.len() <= 2 { continue; }
            let score: i64 = c.iter().map(|n| *priorities.get(n).unwrap_or(&0) as i64).sum();
            if score > best_clique_score {
                best_clique_score = score;
                best_clique = c;
            }
        }
        let max_clique = best_clique;
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

    // Ordenar soluciones por score total descendente
    solutions.sort_by(|a, b| b.1.cmp(&a.1));

    // Imprimir resumen ordenado para que los logs reflejen el ranking final
    eprintln!("   Resumen final (ordenado por score):");
    for (i, (sol, total)) in solutions.iter().enumerate() {
        eprintln!("      {}: {} cursos, score {}", i + 1, sol.len(), total);
    }

    solutions
}

/// Resolver ruta crítica considerando SOLO dependencias, SIN verificar conflictos de horarios.
/// Esta versión es útil para obtener la ruta crítica "ideal" en términos de dependencias,
/// sin restricciones de horarios. Útil para validar el orden de cursos correcto.
pub fn get_clique_dependencies_only(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    eprintln!("DEBUG get_clique_dependencies_only: {} secciones, {} ramos disponibles (SIN VERIFICACIÓN DE HORARIOS)", 
              lista_secciones.len(), ramos_disponibles.len());
    println!("=== Generador de Ruta Crítica (Dependencias Solamente) ===");
    println!("Ramos disponibles:\n");
    for (i, (codigo, ramo)) in ramos_disponibles.iter().enumerate() {
        println!("{}.- {} || {}", i, ramo.nombre, codigo);
    }

    // Construir índice inverso PA2025-1 código → clave del HashMap
    let code_to_key = build_code_to_key_index(ramos_disponibles);

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
            // CASO ESPECIAL: Para electivos, buscar por el código de PA2025-1
            if let Some(key) = code_to_key.get(&seccion.codigo) {
                ramos_disponibles.get(key)
            } else {
                eprintln!("WARN: Electivo con código '{}' no encontrado en code_to_key", seccion.codigo);
                None
            }
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

    // CLAVE DIFERENCIA: Conectar TODOS los cursos sin horarios conflictivos
    // Solo verificar código (sin duplicados) pero NO horarios
    for i in 0..node_indices.len() {
        for j in (i + 1)..node_indices.len() {
            let sec_i = &lista_secciones[graph[node_indices[i]]];
            let sec_j = &lista_secciones[graph[node_indices[j]]];

            if sec_i.codigo_box != sec_j.codigo_box &&
               sec_i.codigo[..std::cmp::min(7, sec_i.codigo.len())] != 
               sec_j.codigo[..std::cmp::min(7, sec_j.codigo.len())] {
                // Conectar SIN verificar horarios - solo dependencias
                graph.add_edge(node_indices[i], node_indices[j], ());
            }
        }
    }

    println!("\n=== Soluciones Recomendadas (Dependencias Solamente) ===");

    let mut prev_solutions = Vec::new();
    let mut graph_copy = graph.clone();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    for _solution_num in 1..=5 {
        // Buscar la mejor clique probando varias semillas de tie-breaking
        let mut best_clique: Vec<NodeIndex> = Vec::new();
        let mut best_clique_score: i64 = std::i64::MIN;
        let seed_attempts = 8usize;
        for seed in 0..seed_attempts {
            let c = find_max_weight_clique_with_seed(&graph_copy, &priorities, seed);
            if c.len() <= 2 { continue; }
            let score: i64 = c.iter().map(|n| *priorities.get(n).unwrap_or(&0) as i64).sum();
            if score > best_clique_score {
                best_clique_score = score;
                best_clique = c;
            }
        }
        let max_clique = best_clique;
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

    // Ordenar soluciones por score total descendente
    solutions.sort_by(|a, b| b.1.cmp(&a.1));

    // Imprimir resumen ordenado para consistencia en logs
    eprintln!("   Resumen final (ordenado por score) [dependencies_only]:");
    for (i, (sol, total)) in solutions.iter().enumerate() {
        eprintln!("      {}: {} cursos, score {}", i + 1, sol.len(), total);
    }

    solutions
}

pub fn get_clique_max_pond_with_prefs(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &crate::api_json::InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let mut filtered: Vec<Seccion> = Vec::new();
    // Si el usuario especificó `horarios_preferidos` a nivel de params, aplicamos
    // un filtrado estricto: sólo se permiten secciones que estén completamente
    // contenidas en alguna de las franjas preferidas.
    let prefs = &params.horarios_preferidos;
    for s in lista_secciones.iter() {
        let mut is_taken = false;
        for rp in params.ramos_pasados.iter() {
            if rp == &s.codigo_box || s.codigo.starts_with(rp) { is_taken = true; break; }
        }
        if is_taken { continue; }

        if !prefs.is_empty() {
            let mut any_pref_match = false;
            for pref in prefs.iter() {
                if crate::algorithm::conflict::seccion_contenida_en_rango(s, pref) {
                    any_pref_match = true;
                    break;
                }
            }
            if !any_pref_match { continue; }
        }

        filtered.push(s.clone());
    }

    // Construir índice inverso PA2025-1 código → clave del HashMap (para TODOS los ramos)
    let code_to_name = build_code_to_name_index(ramos_disponibles);
    let code_to_key_electivos = build_code_to_key_index(ramos_disponibles);
    // --- NUEVO: construir sets elegibles por prerrequisitos para horizonte de 2 semestres ---
    use std::collections::HashSet;
    // passed_names: normalizados (usamos code_to_name para mapear códigos a nombres normalizados)
    let mut passed_names: HashSet<String> = HashSet::new();
    for rp in params.ramos_pasados.iter() {
        if let Some(n) = code_to_name.get(rp) {
            passed_names.insert(n.clone());
        } else {
            passed_names.insert(crate::excel::normalize_name(rp));
        }
    }
    // priority maps usados para ajustes manuales (pueden permanecer vacíos)
    let mut priority_ramo: HashMap<String, i32> = HashMap::new();
    let mut priority_sec: HashMap<String, i32> = HashMap::new();
    // Helper: construir prereq_map a partir de `codigo_ref`/`numb_correlativo`.
    // Malla actual sólo guarda una referencia al ramo anterior mediante `codigo_ref`.
    // Mapear numb_correlativo -> clave (nombre normalizado) y usarlo para construir
    // prereq_map: clave -> Vec<clave_prereq>
    // Construir índice numb_correlativo -> clave (por si hay IDs numéricos en la hoja)
    let mut by_numb: HashMap<i32, String> = HashMap::new();
    for (key, ramo) in ramos_disponibles.iter() {
        by_numb.insert(ramo.numb_correlativo, key.clone());
    }

    // Intentar leer prerequisitos desde las hojas adicionales de la malla (preferencia B)
    // La función devuelve: codigo_str -> Vec<codigo_prereq_str>
    let mut prereq_map: HashMap<String, Vec<String>> = HashMap::new();
    match crate::excel::leer_prerequisitos(&params.malla) {
        Ok(sheet_map) if !sheet_map.is_empty() => {
            // Construir índice código_string -> key (considerar campo `codigo` y `numb_correlativo`)
            let mut code_to_key: HashMap<String, String> = HashMap::new();
            for (k, ramo) in ramos_disponibles.iter() {
                if !ramo.codigo.is_empty() {
                    code_to_key.insert(ramo.codigo.clone(), k.clone());
                }
                code_to_key.insert(ramo.numb_correlativo.to_string(), k.clone());
            }

            for (codigo, prereqs) in sheet_map.into_iter() {
                // localizar la clave objetivo a partir del codigo
                if let Some(target_key) = code_to_key.get(&codigo).cloned() {
                    let mut mapped: Vec<String> = Vec::new();
                    for p in prereqs.iter() {
                        if let Some(pk) = code_to_key.get(p) {
                            mapped.push(pk.clone());
                        } else {
                            // intentar mapear por nombre normalizado
                            let pname = crate::excel::normalize_name(p);
                            if ramos_disponibles.contains_key(&pname) {
                                mapped.push(pname);
                            } else if let Ok(pid) = p.parse::<i32>() {
                                if let Some(pk2) = by_numb.get(&pid) {
                                    mapped.push(pk2.clone());
                                }
                            }
                        }
                    }
                    prereq_map.insert(target_key, mapped);
                } else if let Ok(target_id) = codigo.parse::<i32>() {
                    // fallback: buscar por id numérico
                    if let Some(tk) = by_numb.get(&target_id) {
                        let mut mapped: Vec<String> = Vec::new();
                        for p in prereqs.iter() {
                            if let Some(pk) = code_to_key.get(p) {
                                mapped.push(pk.clone());
                            } else {
                                let pname = crate::excel::normalize_name(p);
                                if ramos_disponibles.contains_key(&pname) {
                                    mapped.push(pname);
                                } else if let Ok(pid) = p.parse::<i32>() {
                                    if let Some(pk2) = by_numb.get(&pid) {
                                        mapped.push(pk2.clone());
                                    }
                                }
                            }
                        }
                        prereq_map.insert(tk.clone(), mapped);
                    }
                }
            }
        }
        _ => {
            // Fallback: construir prereq_map a partir de `codigo_ref` si no hay hoja de prereqs
            for (key, ramo) in ramos_disponibles.iter() {
                let mut pvec: Vec<String> = Vec::new();
                if let Some(prev_id) = ramo.codigo_ref {
                    if let Some(prev_key) = by_numb.get(&prev_id) {
                        pvec.push(prev_key.clone());
                    }
                }
                prereq_map.insert(key.clone(), pvec);
            }
        }
    }

    // S1: prereqs ⊆ passed
    let mut s1: HashSet<String> = HashSet::new();
    for (key, _) in ramos_disponibles.iter() {
        let all_passed = match prereq_map.get(key) {
            Some(prs) => prs.iter().all(|pr| passed_names.contains(pr)),
            None => true,
        };
        if all_passed {
            s1.insert(key.clone());
        }
    }
    // S2: prereqs ⊆ passed ∪ S1
    let mut s2: HashSet<String> = HashSet::new();
    let mut passed_plus_s1 = passed_names.clone();
    for k in s1.iter() { passed_plus_s1.insert(k.clone()); }
    for (key, _) in ramos_disponibles.iter() {
        let all_ok = match prereq_map.get(key) {
            Some(prs) => prs.iter().all(|pr| passed_plus_s1.contains(pr)),
            None => true,
        };
        if all_ok && !s1.contains(key) {
            s2.insert(key.clone());
        }
    }
    // Conjuntos listos: sólo permitiremos ramos ∈ (s1 ∪ s2)
    // --- FIN NUEVO ---

    let mut graph = UnGraph::<usize, ()>::new_undirected();
    // Nuevo: cada nodo modela (seccion_idx, semestre) donde semestre = 1 o 2
    // node_meta[node_index.index()] = (seccion_idx, semestre)
    let mut node_meta: Vec<(usize, u8)> = Vec::new();
    let mut priorities = HashMap::new();

    for (idx, seccion) in filtered.iter().enumerate() {
        // Buscar por nombre normalizado (para NO-ELECTIVOS)
        let nombre_norm = crate::excel::normalize_name(&seccion.nombre);
        
        // resolver RamoDisponible
        let ramo_key_opt = if let Some(_) = ramos_disponibles.get(&nombre_norm) {
            Some(nombre_norm.clone())
        } else if nombre_norm == "electivo profesional" {
            // CASO ESPECIAL: Para electivos, buscar por el código de PA2025-1
            if let Some(key) = code_to_key_electivos.get(&seccion.codigo) {
                Some(key.clone())
            } else {
                eprintln!("WARN: Electivo con código '{}' no encontrado en code_to_key", seccion.codigo);
                None
            }
        } else {
            None
        };

        let ramo_key = match ramo_key_opt {
            Some(k) => k,
            None => {
                eprintln!("WARN: No se encontró ramo con nombre normalizado '{}' (original: '{}', código: '{}')", nombre_norm, seccion.nombre, seccion.codigo);
                continue;
            }
        };

        // Resolver referencia al RamoDisponible correspondiente
        let ramo = match ramos_disponibles.get(&ramo_key) {
            Some(r) => r,
            None => {
                eprintln!("WARN: clave '{}' no encontrada en ramos_disponibles (debería existir)", ramo_key);
                continue;
            }
        };

        // Solo incluir si el ramo pertenece a S1 o S2 (prune por prerrequisitos)
        let is_s1 = s1.contains(&ramo_key);
        let is_s2 = s2.contains(&ramo_key);
        if !is_s1 && !is_s2 {
            // no es elegible dentro de horizonte 2 semestres
            continue;
        }

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

        let mut horario_boost: i32 = 0;

        // dias_set usado en varios checks; inicializar y rellenar antes de posibles usos
        use std::collections::HashSet as _HashSet;
        let mut dias_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Referencia a filtros para chequear nuevos filtros añadidos
        let filtros_opt = params.filtros.as_ref();

        // Si el usuario pidió días libres explícitos, excluimos secciones que ocurran en esos días
        if let Some(filtros) = filtros_opt {
            if let Some(dhl) = &filtros.dias_horarios_libres {
                if dhl.habilitado {
                    // 1) dias_libres_preferidos: si la sección ocurre en esos días la excluimos
                    if let Some(dias_pref) = &dhl.dias_libres_preferidos {
                        // Construir set de días presentes en la sección
                        dias_set.clear();
                        for hstr in seccion.horario.iter() {
                            let token = hstr.split_whitespace().next().unwrap_or("").to_uppercase();
                            if token.len() >= 2 {
                                dias_set.insert(token.chars().take(2).collect());
                            }
                        }

                        let mut intersects = false;
                        for d in dias_pref.iter() {
                            let dnorm = d.trim().to_uppercase();
                            let pref_day = match dnorm.as_str() {
                                "LUN" | "LU" | "LUNES" => "LU",
                                "MAR" | "MA" | "MARTES" => "MA",
                                "MIE" | "MI" | "MIERCOLES" => "MI",
                                "JUE" | "JU" | "JUEVES" => "JU",
                                "VIE" | "VI" | "VIERNES" => "VI",
                                "SAB" | "SA" | "SABADO" => "SA",
                                "DOM" | "DO" | "DOMINGO" => "DO",
                                other => other,
                            };
                            if dias_set.contains(pref_day) {
                                intersects = true;
                                break;
                            }
                        }
                        if intersects { continue; }
                    }

                    // 2) franjas_prohibidas: si la sección solapa con cualquiera, la excluimos
                    if let Some(franjas) = &dhl.franjas_prohibidas {
                        let mut prohibited = false;
                            for fran in franjas.iter() {
                                // Usar el parser robusto: tratamos la franja prohibida como un horario
                                // y preguntamos si la sección solapa con ella.
                                let fran_vec = vec![fran.clone()];
                                if horarios_tienen_conflicto(&seccion.horario, &fran_vec) {
                                    prohibited = true;
                                    break;
                                }
                                // Fallback: comprobar token de día como heurística rápida
                                let fran_up = fran.to_uppercase();
                                let day_token = fran_up.split_whitespace().next().unwrap_or("");
                                if !day_token.is_empty() {
                                    for hstr in seccion.horario.iter() {
                                        if hstr.to_uppercase().starts_with(day_token) {
                                            prohibited = true; break;
                                        }
                                    }
                                }
                                if prohibited { break; }
                            }
                        if prohibited { continue; }
                    }

                    // 3) no_sin_horario: si está marcado, evitamos secciones "Sin horario"
                    if dhl.no_sin_horario.unwrap_or(false) {
                        let mut has_sin = false;
                        for hstr in seccion.horario.iter() {
                            if hstr.to_lowercase().contains("sin horario") {
                                has_sin = true; break;
                            }
                        }
                        if has_sin { continue; }
                    }
                }
            }
        }

        // Boost por rangos horarios preferidos
        for pref in params.horarios_preferidos.iter() {
            for h in seccion.horario.iter() {
                if h.contains(pref) || pref.contains(h) {
                    horario_boost += 2000;
                    break;
                }
            }
            if horario_boost > 0 { break; }
        }

        // Factor dificultad: `ramo.dificultad` = % reprobados (0..100).
        // Usamos (100 - dificultad) para dar mayor bonus a cursos con más aprobados.
        let dd = if let Some(dif_reprobados) = ramo.dificultad {
            ((100.0 - dif_reprobados) / 10.0) as i32
        } else { 5 };

        // Aplicar filtros opcionales restantes
        if let Some(filtros) = params.filtros.as_ref() {
            // Días/horarios libres (minimizar ventanas)
            if let Some(dhl) = &filtros.dias_horarios_libres {
                if dhl.habilitado {
                    if dhl.minimizar_ventanas.unwrap_or(false) {
                            let days_count = dias_set.len() as i32;
                            if days_count > 2 {
                                horario_boost -= 500 * (days_count - 2);
                            }
                        }
                }
            }

            // Preferencias de profesores
            if let Some(prefp) = &filtros.preferencias_profesores {
                if prefp.habilitado {
                    let profesor_lower = seccion.profesor.to_lowercase();

                    // Si el usuario proporcionó una lista explícita de profesores preferidos,
                    // la semántica estricta es: sólo permitir secciones cuyo profesor esté en esa lista.
                    if let Some(pref_list) = &prefp.profesores_preferidos {
                        if !pref_list.is_empty() {
                            let mut matched = false;
                            for p in pref_list.iter() {
                                if !p.is_empty() && profesor_lower.contains(&p.to_lowercase()) {
                                    matched = true;
                                    break;
                                }
                            }
                            if !matched { continue; } // excluir sección si no coincide con preferred list
                        }
                    }

                    // Si el usuario proporcionó profesores a evitar, excluimos secciones cuyo profesor coincida
                    if let Some(avoid_list) = &prefp.profesores_evitar {
                        if !avoid_list.is_empty() {
                            let mut avoid = false;
                            for p in avoid_list.iter() {
                                if !p.is_empty() && profesor_lower.contains(&p.to_lowercase()) {
                                    avoid = true; break;
                                }
                            }
                            if avoid { continue; }
                        }
                    }

                    // Si llegamos aquí, no se excluyó: aplicar boosts/penalizaciones suaves como antes
                    if let Some(pref_list) = &prefp.profesores_preferidos {
                        for p in pref_list.iter() {
                            if !p.is_empty() && profesor_lower.contains(&p.to_lowercase()) {
                                horario_boost += 3000;
                                break;
                            }
                        }
                    }
                    if let Some(avoid_list) = &prefp.profesores_evitar {
                        for p in avoid_list.iter() {
                            if !p.is_empty() && profesor_lower.contains(&p.to_lowercase()) {
                                horario_boost -= 3000;
                                break;
                            }
                        }
                    }
                }
            }
        }

        let prioridad = cc * 10000 + uu * 1000 + kk * 100 + ss * 10 + dd + horario_boost;
        // si pertenece a S1, añadir nodo para semestre 1
        if is_s1 {
            let n = graph.add_node(node_meta.len());
            node_meta.push((idx, 1u8));
            priorities.insert(n, prioridad);
        }
        // si pertenece a S2, añadir nodo para semestre 2
        if is_s2 {
            let n = graph.add_node(node_meta.len());
            node_meta.push((idx, 2u8));
            // dar prioridad ligeramente inferior a S1 para mismo curso por heurística (opcional)
            priorities.insert(n, prioridad - 1);
        }
    }

    // Conectar nodos: secciones compatibles y sin conflicto horario.
    // node_meta índice corresponde al payload asociado al node index (node.index()).
    for a in 0..node_meta.len() {
        for b in (a + 1)..node_meta.len() {
            let (sec_a_idx, sem_a) = node_meta[a];
            let (sec_b_idx, sem_b) = node_meta[b];
            let sec_a = &filtered[sec_a_idx];
            let sec_b = &filtered[sec_b_idx];

            // mismo código_box -> no emparejar
            if sec_a.codigo_box == sec_b.codigo_box { continue; }
            if sec_a.codigo[..std::cmp::min(7, sec_a.codigo.len())] ==
               sec_b.codigo[..std::cmp::min(7, sec_b.codigo.len())] { continue; }

            // si hay conflicto horario entre las secciones -> no conectar
            if horarios_tienen_conflicto(&sec_a.horario, &sec_b.horario) { continue; }

            // agregar arista entre nodos a y b (compatibles)
            graph.add_edge(NodeIndex::new(a), NodeIndex::new(b), ());
        }
    }

    let mut prev_solutions = Vec::new();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    eprintln!("\n📊 [get_clique_with_user_prefs] Iniciando búsqueda de múltiples soluciones");
    eprintln!("   Grafo: {} nodos, {} aristas", graph.node_count(), graph.edge_count());

    let max_iterations = 8;

    let total_start = Instant::now();

    for iteration in 1..=max_iterations {
        let iter_start = Instant::now();
        // Probar múltiples seeds para aumentar probabilidad de hallar la mejor clique
        let mut best_clique: Vec<NodeIndex> = Vec::new();
        let mut best_score: i64 = std::i64::MIN;
        let seed_attempts = 8usize;
        for seed in 0..seed_attempts {
            let c = find_max_weight_clique_with_seed(&graph, &priorities, seed);
            if c.len() <= 2 { continue; }
            let score: i64 = c.iter().map(|n| *priorities.get(n).unwrap_or(&0) as i64).sum();
            if score > best_score {
                best_score = score;
                best_clique = c;
            }
        }
        let max_clique = best_clique;
        if max_clique.len() <= 2 {
            eprintln!("   Iter {}: Clique muy pequeño ({}), deteniendo", iteration, max_clique.len());
            break;
        }

        eprintln!("   Iter {}: Clique de {} nodos encontrado", iteration, max_clique.len());

        // Determinar número máximo de ramos permitidos por solución (cap fijo a 6)
        let max_ramos: usize = 6;

        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();

        // 🔧 Sort ASCENDING (lowest priority first) like Python version
        arr_aux_delete.sort_by_key(|&(_, prio)| prio);
        while arr_aux_delete.len() > max_ramos { arr_aux_delete.remove(0); }  // Remove lowest priority nodes

        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        if prev_solutions.contains(&solution_key) {
            let iter_elapsed = iter_start.elapsed();
            eprintln!("      -> Solución duplicada, penalizando nodos (iter tiempo: {:.3}s)", iter_elapsed.as_secs_f64());
            // 🔧 Penalize used nodes instead of removing them
            for &(node_idx, _) in &arr_aux_delete {
                if let Some(prio) = priorities.get_mut(&node_idx) {
                    *prio = (*prio / 2).max(100);  // Reduce priority to half
                }
            }
            continue;
        }

        // Construir entries con info de semestre y validar prerrequisitos dentro de la clique
        let mut solution_entries: Vec<(Seccion, i32, u8)> = Vec::new();
        let mut total_score_i64: i64 = 0;
        // mapear ramos seleccionados en semestre 1 (por clave del ramo en ramos_disponibles)
        let mut selected_s1_ramos: HashSet<String> = HashSet::new();
        for &(node_idx, prioridad) in &arr_aux_delete {
            let (seccion_idx, sem) = node_meta[node_idx.index()];
            let seccion = filtered[seccion_idx].clone();
            // resolver clave de ramo normalizada
            let clave = crate::excel::normalize_name(&seccion.nombre);
            if sem == 1 { selected_s1_ramos.insert(clave.clone()); }
            solution_entries.push((seccion, prioridad, sem));
            total_score_i64 += prioridad as i64;
        }

        // Validar: para cada nodo en S2, sus prereqs deben estar en passed ∪ selected_s1_ramos
        let mut prereq_ok = true;
        for (_sec, _prio, sem) in solution_entries.iter() {
            if *sem == 2 {
                // obtener clave del ramo para esta sección
                // implementación práctica: revisar cada entry con sem==2
                // (se usa el nombre de la sección para mapear a la clave en ramos_disponibles)
                // Esto ya se hace abajo en el loop: replicamos aquí
            }
        }
        // implementación práctica: revisar cada entry con sem==2
        for (sec, _prio, sem) in solution_entries.iter() {
            if *sem == 2 {
                let ram_key = crate::excel::normalize_name(&sec.nombre);
                let needed_slice: &[String] = match prereq_map.get(&ram_key) {
                    Some(v) => v.as_slice(),
                    None => &[],
                };
                for pr in needed_slice.iter() {
                    if !(passed_names.contains(pr) || selected_s1_ramos.contains(pr)) {
                        prereq_ok = false;
                        break;
                    }
                }
                if !prereq_ok { break; }
            }
        }

        if !prereq_ok {
            eprintln!("      -> Solución descartada: requisitos semestrales no cumplidos (prerrequisitos S2 faltantes)");
            // penalizar nodos de la clique para no repetirla
            for &(node_idx, _) in &arr_aux_delete {
                if let Some(prio) = priorities.get_mut(&node_idx) {
                    *prio = (*prio / 2).max(100);
                }
            }
            prev_solutions.push(solution_key);
            continue;
        }

        let mut accept_solution = true;
        if let Some(filtros) = params.filtros.as_ref() {
            if let Some(balance) = filtros.balance_lineas.as_ref() {
                if balance.habilitado {
                    if let Some(ref lineas_map) = balance.lineas {
                        // Construir mapa de conteos reales por línea para la solución
                        use std::collections::HashMap as Map;
                        let mut reales: Map<String, usize> = Map::new();
                        let mut total_selected: usize = 0;

                        for (sec, _prio, _sem) in solution_entries.iter() {
                            // Resolver RamoDisponible a partir de la sección (mismo heurístico usado antes)
                            let nombre_norm = crate::excel::normalize_name(&sec.nombre);
                            let ramo_opt = if let Some(r) = ramos_disponibles.get(&nombre_norm) {
                                Some(r)
                            } else if nombre_norm == "electivo profesional" {
                                // buscar por código entre electivos
                                // usamos el mismo builder como heurística: buscar clave exacta
                                // Si no encontramos, marcamos como sin línea y esto causará rechazo
                                None
                            } else {
                                None
                            };

                            if let Some(ramo) = ramo_opt {
                                // mapear ramo.nombre a alguna línea provista en `lineas_map` por substring
                                let rname = ramo.nombre.to_lowercase();
                                let mut matched = false;
                                for key in lineas_map.keys() {
                                    if rname.contains(&key.to_lowercase()) {
                                        *reales.entry(key.clone()).or_insert(0) += 1;
                                        matched = true;
                                        break;
                                    }
                                }
                                // si no matchea ninguna línea, considerarlo incumplimiento estricto
                                if !matched {
                                    accept_solution = false;
                                    break;
                                }
                                total_selected += 1;
                            } else {
                                // No pude mapear la sección al ramo; tratar como incumplimiento
                                accept_solution = false;
                                break;
                            }
                        }

                        if accept_solution {
                            // Si no hay ramos seleccionados (ej: 0), entonces no cumple
                            if total_selected == 0 {
                                accept_solution = false;
                            } else {
                                // Calcular expected counts a partir de porcentajes y total_selected
                                // Algoritmo: asignar floor(p * total), luego distribuir residuos por mayor fracción
                                use std::cmp::Ordering;
                                let mut expected: Map<String, usize> = Map::new();
                                let mut frac_parts: Vec<(_, f64)> = Vec::new();
                                let mut assigned: usize = 0;
                                for (k, v) in lineas_map.iter() {
                                    let exact = v * (total_selected as f64);
                                    let base = exact.floor() as usize;
                                    expected.insert(k.clone(), base);
                                    assigned += base;
                                    frac_parts.push((k.clone(), exact - (base as f64)));
                                }
                                let mut remaining = total_selected.saturating_sub(assigned);
                                // ordenar por parte fraccional descendente
                                frac_parts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
                                let mut idx = 0;
                                while remaining > 0 && !frac_parts.is_empty() {
                                    let key = &frac_parts[idx % frac_parts.len()].0;
                                    *expected.entry(key.clone()).or_insert(0) += 1;
                                    remaining -= 1;
                                    idx += 1;
                                }

                                // Ahora comparar expected con reales exactamente
                                for (k, &exp_count) in expected.iter() {
                                    let real_count = *reales.get(k).unwrap_or(&0);
                                    if real_count != exp_count {
                                        accept_solution = false;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

    let iter_elapsed = iter_start.elapsed();
    if accept_solution {
        eprintln!("      -> Solución {} aceptada ({} cursos, score {}, tiempo: {:.3}s)", solutions.len() + 1, arr_aux_delete.len(), total_score_i64, iter_elapsed.as_secs_f64());

        // Convertir entries (Seccion, i32, semestre) -> (Seccion, i32) para la API
        let simple_entries: Vec<(Seccion, i32)> = solution_entries.iter()
            .map(|(s, p, _sem)| (s.clone(), *p))
            .collect();

        solutions.push((simple_entries, total_score_i64));
    } else {
        eprintln!("      -> Solución descartada por filtros estrictos (balance_lineas u otros) (tiempo: {:.3}s)", iter_elapsed.as_secs_f64());
        // Penalizar nodos usados para evitar elegir la misma composición repetidamente
        for &(node_idx, _) in &arr_aux_delete {
            if let Some(prio) = priorities.get_mut(&node_idx) {
                *prio = (*prio / 2).max(100);
            }
        }
        // No push; continuar buscando otras soluciones
    }
        prev_solutions.push(solution_key);

        // 🔧 Penalize all nodes in the clique for next iteration
        for &(node_idx, _) in &arr_aux_delete {
            if let Some(prio) = priorities.get_mut(&node_idx) {
                *prio = (*prio / 2).max(100);  // Reduce priority to half
            }
        }
    }

    let total_elapsed = total_start.elapsed();
    eprintln!("   Completado: {} soluciones generadas", solutions.len());
    eprintln!("   Tiempo total búsqueda: {:.3}s", total_elapsed.as_secs_f64());

    // Ordenar soluciones por score total descendente antes de devolver
    solutions.sort_by(|a, b| b.1.cmp(&a.1));
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
