use std::collections::HashMap;
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::algorithm::conflict::horarios_tienen_conflicto;
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

    // Construir índice inverso PA2025-1 código → clave del HashMap (para TODOS los ramos)
    let code_to_name = build_code_to_name_index(ramos_disponibles);
    let code_to_key_electivos = build_code_to_key_index(ramos_disponibles);

    let mut priority_ramo: HashMap<String, i32> = HashMap::new();
    let priority_sec: HashMap<String, i32> = HashMap::new();

    // Convertir ramos_prioritarios de códigos a nombres normalizados
    for rp in params.ramos_prioritarios.iter() {
        // Si es código, convertir a nombre normalizado; si no, usarlo como está
        let nombre_o_codigo = if let Some(nombre_norm) = code_to_name.get(rp) {
            nombre_norm.clone()
        } else {
            rp.clone()  // Si no encuentra en mapeo, asumir que ya es nombre
        };
        priority_ramo.insert(nombre_o_codigo, 5000);
        // También agregar el código directo para casos electivos
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
            // CASO ESPECIAL: Para electivos, buscar por el código de PA2025-1
            if let Some(key) = code_to_key_electivos.get(&seccion.codigo) {
                ramos_disponibles.get(key)
            } else {
                None
            }
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

        // Horario boost y filtros de usuario (Reglas 3 y 5)
        use std::collections::HashSet;
        let mut dias_set: HashSet<String> = HashSet::new();
        // Mejor extracción de días: buscamos abreviaturas al inicio de cada token
        for hstr in seccion.horario.iter() {
            // Ejemplos esperados: "LU 08:30-10:00", "MA 10:30-12:00", "LUN 08:30-10:00"
            let first = hstr.split_whitespace().next().unwrap_or("");
            let token = first.trim_matches(|c: char| !c.is_alphanumeric()).to_uppercase();
            if token.len() >= 2 && token.len() <= 4 {
                // Normalizar LUN->LU, MIE->MI, JUE->JU, vie->VI
                let day = match &token[..] {
                    t if t.starts_with("LU") => "LU",
                    t if t.starts_with("MA") => "MA",
                    t if t.starts_with("MI") => "MI",
                    t if t.starts_with("JU") => "JU",
                    t if t.starts_with("VI") => "VI",
                    t if t.starts_with("SA") => "SA",
                    t if t.starts_with("DO") => "DO",
                    _ => "",
                };
                if !day.is_empty() {
                    dias_set.insert(day.to_string());
                }
            }
        }

        let mut horario_boost: i32 = 0;

        // Referencia a filtros para chequear nuevos filtros añadidos
        let filtros_opt = params.filtros.as_ref();

        // Si el usuario pidió días libres explícitos, excluimos secciones que ocurran en esos días
        if let Some(filtros) = filtros_opt {
            if let Some(dhl) = &filtros.dias_horarios_libres {
                if dhl.habilitado {
                    // 1) dias_libres_preferidos: si la sección ocurre en esos días la excluimos
                    if let Some(dias_pref) = &dhl.dias_libres_preferidos {
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
                            let fran_up = fran.to_uppercase();
                            for hstr in seccion.horario.iter() {
                                let h_up = hstr.to_uppercase();
                                if h_up.contains(&fran_up) || fran_up.contains(&h_up) {
                                    prohibited = true; break;
                                }
                                // Also check simple day token match (e.g., "MI" in "MI 10:00")
                                let day_token = fran_up.split_whitespace().next().unwrap_or("");
                                if !day_token.is_empty() && h_up.starts_with(day_token) {
                                    prohibited = true; break;
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
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    eprintln!("\n📊 [get_clique_with_user_prefs] Iniciando búsqueda de múltiples soluciones");
    eprintln!("   Grafo: {} nodos, {} aristas", graph.node_count(), graph.edge_count());

    let max_iterations = 8;

    let total_start = Instant::now();

    for iteration in 1..=max_iterations {
        let iter_start = Instant::now();
        let max_clique = find_max_weight_clique(&graph, &priorities);
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

        let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
        let mut total_score_i64: i64 = 0;

        for &(node_idx, prioridad) in &arr_aux_delete {
            let seccion_idx = graph[node_idx];
            let seccion = filtered[seccion_idx].clone();
            solution_entries.push((seccion, prioridad));
            total_score_i64 += prioridad as i64;
        }

    let iter_elapsed = iter_start.elapsed();
    eprintln!("      -> Solución {} aceptada ({} cursos, score {}, tiempo: {:.3}s)", solutions.len() + 1, arr_aux_delete.len(), total_score_i64, iter_elapsed.as_secs_f64());

        solutions.push((solution_entries, total_score_i64));
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
