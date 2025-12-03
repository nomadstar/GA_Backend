use std::collections::HashMap;
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::algorithm::conflict::horarios_tienen_conflicto;
use std::time::Instant;
use std::env;
use calamine::Reader;
use std::collections::HashSet;

// Helper local: convertir calamine::Data a String (similar a excel::io::data_to_string)
fn excel_data_to_string(d: &calamine::Data) -> String {
    match d {
        calamine::Data::String(s) => s.trim().to_string(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        calamine::Data::Empty => String::new(),
        calamine::Data::Error(_) => String::new(),
        calamine::Data::DateTime(s) => s.to_string(),
        calamine::Data::DateTimeIso(s) => s.clone(),
        calamine::Data::DurationIso(s) => s.clone(),
    }
}

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

    let mut prev_solutions: Vec<Vec<NodeIndex>> = Vec::new();
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
    eprintln!("DEBUG get_clique_dependencies_only: {} secciones, {} ramos disponibles (SIN VERIFICACIÓN DE HORARIOS)", lista_secciones.len(), ramos_disponibles.len());
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

    let mut prev_solutions: Vec<Vec<NodeIndex>> = Vec::new();
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
    eprintln!("🔧 DEBUG: get_clique_max_pond_with_prefs iniciada");
    
    // Construir índices necesarios PRIMERO
    let code_to_name = build_code_to_name_index(ramos_disponibles);
    let code_to_key_electivos = build_code_to_key_index(ramos_disponibles);

    // (PERT integration moved to `ruta.rs` orchestrator). Use the provided
    // `ramos_disponibles` map directly here; ruta.rs will run PERT and then
    // call these planner functions when PERT-based updates are required.
    // --------------------------------------------------------------------

    // PASO 1: Calcular max_passed_semester ANTES de filtrar
    let mut max_passed_semester: i32 = 0;
    for passed_code in params.ramos_pasados.iter() {
        let passed_norm = crate::excel::normalize_name(passed_code);
        // Intentar encontrar por clave normalizada
        if let Some(r) = ramos_disponibles.get(&passed_norm) {
            if let Some(sem) = r.semestre {
                max_passed_semester = std::cmp::max(max_passed_semester, sem);
                continue;
            }
        }
        // Intentar encontrar por campo `codigo` dentro de los ramos_disponibles
        for (_k, r) in ramos_disponibles.iter() {
            if r.codigo == *passed_code {
                if let Some(sem) = r.semestre {
                    max_passed_semester = std::cmp::max(max_passed_semester, sem);
                }
                break;
            }
        }
    }
    let max_allowed_semester = max_passed_semester + 2;
    eprintln!("📊 Horizonte de semestres: max_pasado={}, max_permitido={}", max_passed_semester, max_allowed_semester);

    // PASO 2: Filtrar secciones por semestre permitido + ramos ya pasados + horarios_preferidos
    let mut filtered: Vec<Seccion> = Vec::new();
    for s in lista_secciones.iter() {
        // Excluir si ya fue aprobado (por codigo_box o coincidencia con ramos_pasados)
        let mut is_taken = false;
        for rp in params.ramos_pasados.iter() {
            if rp == &s.codigo_box || s.codigo.starts_with(rp) {
                is_taken = true;
                break;
            }
        }
        if is_taken { continue; }

        // Resolver semestre del ramo de la sección (si existe)
        let nombre_norm = crate::excel::normalize_name(&s.nombre);
        let ramo_opt = ramos_disponibles.get(&nombre_norm)
            .or_else(|| ramos_disponibles.values().find(|r| r.codigo == s.codigo));

        if let Some(ramo) = ramo_opt {
            if let Some(sem) = ramo.semestre {
                if sem > max_allowed_semester {
                    // Fuera del horizonte de 2 semestres
                    continue;
                }
            }
        }

        // Aplicar horarios_preferidos (si provistos): si hay preferencia, la sección debe encajar en alguna
        if !params.horarios_preferidos.is_empty() {
            let mut any_pref_match = false;
            for pref in params.horarios_preferidos.iter() {
                if crate::algorithm::conflict::seccion_contenida_en_rango(s, pref) {
                    any_pref_match = true;
                    break;
                }
            }
            if !any_pref_match { continue; }
        }

        filtered.push(s.clone());
    }

    eprintln!("   Secciones disponibles después de filtrado: {} (total: {})", filtered.len(), lista_secciones.len());
    
    // PASO 3: Construir passed_names y estructuras de apoyo
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
    let mut by_numb: HashMap<i32, String> = HashMap::new();
    for (key, ramo) in ramos_disponibles.iter() {
        by_numb.insert(ramo.numb_correlativo, key.clone());
    }

    eprintln!("DEBUG: ramos_disponibles.len = {}", ramos_disponibles.len());

    // Intentar leer prerequisitos desde las hojas adicionales de la malla
    let mut prereq_map: HashMap<String, Vec<String>> = HashMap::new();
    let prereq_result = crate::excel::leer_prerequisitos(&params.malla);
    match &prereq_result {
        Ok(m) => eprintln!("DEBUG: leer_prerequisitos OK, sheets_count={}", m.len()),
        Err(e) => eprintln!("DEBUG: leer_prerequisitos ERR: {:?}", e),
    }

    match prereq_result {
        Ok(sheet_map) if !sheet_map.is_empty() => {
            let mut code_to_key: HashMap<String, String> = HashMap::new();
            for (k, ramo) in ramos_disponibles.iter() {
                if !ramo.codigo.is_empty() {
                    code_to_key.insert(ramo.codigo.clone(), k.clone());
                }
                code_to_key.insert(crate::excel::normalize_name(&ramo.nombre), k.clone());
                code_to_key.insert(ramo.numb_correlativo.to_string(), k.clone());
            }

            for (codigo, prereqs) in sheet_map.into_iter() {
                let codigo_trim = codigo.trim();
                let mut target_key_opt: Option<String> = None;
                if let Some(k) = code_to_key.get(codigo_trim) { target_key_opt = Some(k.clone()); }
                if target_key_opt.is_none() {
                    let c_norm = crate::excel::normalize_name(codigo_trim);
                    if let Some(k) = code_to_key.get(&c_norm) { target_key_opt = Some(k.clone()); }
                }
                if target_key_opt.is_none() {
                    if let Ok(idn) = codigo_trim.parse::<i32>() {
                        if let Some(k) = by_numb.get(&idn) { target_key_opt = Some(k.clone()); }
                    }
                }

                let mut mapped: Vec<String> = Vec::new();
                for p in prereqs.iter() {
                    let token = p.trim();
                    if token.is_empty() { continue; }
                    if token.chars().all(|c| c == '-' || c == '—' || c == '–' || c.is_whitespace()) {
                        continue;
                    }
                    let parts: Vec<&str> = token.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).collect();
                    if !parts.is_empty() {
                        for seg in parts {
                            if let Ok(pid) = seg.parse::<i32>() {
                                if let Some(k) = by_numb.get(&pid) { mapped.push(k.clone()); continue; }
                            }
                        }
                        if !mapped.is_empty() { continue; }
                    }
                    let token_norm = crate::excel::normalize_name(token);
                    if ramos_disponibles.contains_key(&token_norm) { mapped.push(token_norm); continue; }
                    if let Some(k) = code_to_key.get(token) { mapped.push(k.clone()); continue; }
                    for (rk, r) in ramos_disponibles.iter() {
                        if r.codigo == token || crate::excel::normalize_name(&r.nombre) == token_norm {
                            mapped.push(rk.clone()); break;
                        }
                    }
                }

                if let Some(tk) = target_key_opt {
                    if !mapped.is_empty() {
                        prereq_map.insert(tk, mapped);
                    }
                }
            }
        }
        _ => {
            for (key, ramo) in ramos_disponibles.iter() {
                let mut pvec: Vec<String> = Vec::new();
                if let Some(prev_id) = ramo.codigo_ref {
                    if let Some(prev_key) = by_numb.get(&prev_id) {
                        pvec.push(prev_key.clone());
                    }
                }
                if !pvec.is_empty() {
                    prereq_map.insert(key.clone(), pvec);
                }
            }
        }
    }

    eprintln!("DEBUG prereq_map: total_entries={}, entries_with_no_prereqs={}, entries_with_prereqs={}",
              prereq_map.len(),
              prereq_map.iter().filter(|(_, v)| v.is_empty()).count(),
              prereq_map.iter().filter(|(_, v)| !v.is_empty()).count());

    // Si no conseguimos construir prereq_map (mapa vacío), intentar parsear la
    // hoja principal de la malla buscando la columna "Requisitos" como fallback.
    if prereq_map.is_empty() {
        eprintln!("DEBUG: prereq_map vacío — intentando fallback: parsear columna 'Requisitos' desde la malla");
        if let Ok((malla_path, _oferta, _porc)) = crate::excel::resolve_datafile_paths(&params.malla) {
            if let Ok(mut workbook) = calamine::open_workbook_auto(malla_path.to_str().unwrap_or("")) {
                let sheet_names = workbook.sheet_names().to_owned();
                if !sheet_names.is_empty() {
                    // Preferir la hoja indicada por params.sheet si existe
                    let hoja = if let Some(sheet_name) = params.sheet.as_ref() {
                        if sheet_names.iter().any(|s| s == sheet_name) { sheet_name.clone() } else { sheet_names[0].clone() }
                    } else {
                        // intentar 'Malla' si existe
                        if sheet_names.iter().any(|s| s.to_lowercase().contains("malla")) {
                            sheet_names.iter().find(|s| s.to_lowercase().contains("malla")).unwrap().clone()
                        } else { sheet_names[0].clone() }
                    };

                    if let Ok(range) = workbook.worksheet_range(&hoja) {
                        // detectar columnas: nombre y requisitos
                        let mut name_col: usize = 0;
                        let mut req_col: usize = 3; // heurística: suele estar en la columna 3
                        let rows: Vec<_> = range.rows().collect();
                        if !rows.is_empty() {
                            let header = rows[0];
                            for (i, cell) in header.iter().enumerate() {
                                let s = excel_data_to_string(cell).to_lowercase();
                                if s.contains("nombre") || s.contains("asignatura") || s.contains("curso") {
                                    name_col = i;
                                }
                                if s.contains("requisito") || s.contains("requisitos") {
                                    req_col = i;
                                }
                            }
                        }

                        for (row_idx, row) in range.rows().enumerate() {
                            if row_idx == 0 { continue; }
                            let raw_name = excel_data_to_string(row.get(name_col).unwrap_or(&calamine::Data::Empty)).trim().to_string();
                            let raw_id = excel_data_to_string(row.get(1).unwrap_or(&calamine::Data::Empty)).trim().to_string();
                            let raw_reqs = excel_data_to_string(row.get(req_col).unwrap_or(&calamine::Data::Empty)).trim().to_string();
                            if raw_name.is_empty() { continue; }
                            // determinar clave de target
                            let target_key = if ramos_disponibles.contains_key(&crate::excel::normalize_name(&raw_name)) {
                                crate::excel::normalize_name(&raw_name)
                            } else if let Ok(idn) = raw_id.parse::<i32>() {
                                if let Some(k) = by_numb.get(&idn) { k.clone() } else { crate::excel::normalize_name(&raw_name) }
                            } else { crate::excel::normalize_name(&raw_name) };

                            if raw_reqs.is_empty() {
                                // Campo vacío en la columna 'Requisitos' -> tratar como
                                // ausencia de información (no insertar mapeo). Esto
                                // evita interpretar una celda en blanco como "sin
                                // prerrequisitos".
                                continue;
                            }
                            // split tokens
                            let mut mapped: Vec<String> = Vec::new();
                            for token in raw_reqs.split(|c| c==',' || c==';') {
                                let t = token.trim();
                                if t.is_empty() { continue; }
                                // intentar mapear por numb, codigo o nombre normalizado
                                if let Ok(pid) = t.parse::<i32>() {
                                    if let Some(k) = by_numb.get(&pid) { mapped.push(k.clone()); continue; }
                                }
                                let nrm = crate::excel::normalize_name(t);
                                if ramos_disponibles.contains_key(&nrm) { mapped.push(nrm); continue; }
                                // intentar mapear por codigo exacto (campo `codigo` en RamoDisponible)
                                for (k, r) in ramos_disponibles.iter() {
                                    if r.codigo == t { mapped.push(k.clone()); break; }
                                }
                            }
                            // Sólo insertar si logramos mapear al menos un prerrequisito
                            // explícito. Si no hay mapeos, dejamos la entrada ausente
                            // para indicar 'desconocido'.
                            if !mapped.is_empty() {
                                prereq_map.insert(target_key, mapped);
                            }
                        }
                    }
                }
            }
        }

        eprintln!("DEBUG after fallback prereq_map: total_entries={}", prereq_map.len());
    }

    // Calcular el semestre máximo que ha pasado el usuario
    // Si no ha pasado nada, max_passed_semester = 0 (primer semestre)
    // Si ha pasado algo de S2, max_passed_semester = 2
    // etc.
    let mut max_passed_semester: i32 = 0;
    for passed_ramo_key in passed_names.iter() {
        if let Some(ramo) = ramos_disponibles.values().find(|r| {
            crate::excel::normalize_name(&r.nombre) == *passed_ramo_key
        }) {
            if let Some(sem) = ramo.semestre {
                max_passed_semester = std::cmp::max(max_passed_semester, sem);
            }
        }
    }
    
    // Restricción: máximo 2 semestres de diferencia
    let max_allowed_semester = max_passed_semester + 2;
    eprintln!("DEBUG: max_passed_semester={}, max_allowed_semester={}", max_passed_semester, max_allowed_semester);
    
    // S1 + S2 elegibles según prerequisitos y horizonte de semestres
    // POLÍTICA REVISADA (permite S1 Y S2 para estudiantes nuevos, con restricción de 2 semestres):
    //   - Restricción: solo ramos hasta semestre (max_passed_semester + 2)
    //   - Si ramo_pasados=[] (estudiante nuevo):
    //     - Incluir TODOS los ramos de S1 (sin prerequisitos)
    //     - Incluir TODOS los ramos de S2 (sus prereqs son de S1)
    //   - Si ramo_pasados != []:
    //     - Solo incluir ramos cuyos TODOS los prerequisitos están aprobados
    //   - Si sin info (None): NO elegible (no sabemos si tiene prereqs)
    let mut s1: HashSet<String> = HashSet::new();
    
    for (key, ramo) in ramos_disponibles.iter() {
        // Verificar restricción de semestre
        let within_horizon = if let Some(sem) = ramo.semestre {
            sem <= max_allowed_semester
        } else {
            // Sin información de semestre: asumir que está permitido (ser permisivo)
            true
        };
        
        if !within_horizon {
            continue;  // Fuera del horizonte de 2 semestres
        }
        
        let eligible = if params.ramos_pasados.is_empty() {
            // Estudiante nuevo: permitir S1 (semestre 1) y S2 (semestre 2)
            if let Some(sem) = ramo.semestre {
                sem == 1 || sem == 2
            } else {
                // Sin información de semestre: usar política anterior (solo si tiene prereqs explícitos sin requerir)
                match prereq_map.get(key) {
                    Some(prs) => prs.is_empty(),
                    None => false,
                }
            }
        } else {
            // Estudiante con ramos aprobados: política estricta
            match prereq_map.get(key) {
                Some(prs) => {
                    if prs.is_empty() {
                        true  // Sin prerequisitos
                    } else {
                        prs.iter().all(|pr| passed_names.contains(pr))  // Todos los prereqs satisfechos
                    }
                }
                None => false,  // Sin información = no elegible
            }
        };
        if eligible {
            s1.insert(key.clone());
        }
    }
    // NO construir S2: política estricta de que un ramo sólo puede ser tomado si
    // todos sus prerrequisitos ya están aprobados (en `ramos_pasados`).

    let mut graph = UnGraph::<usize, ()>::new_undirected();
    // Nuevo: cada nodo modela (seccion_idx, semestre, ramo_key)
    // node_meta[node_index.index()] = (seccion_idx, semestre, ramo_key)
    let mut node_meta: Vec<(usize, u8, String)> = Vec::new();
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

        // Solo incluir si el ramo pertenece a S1 (prerrequisitos ya aprobados).
        let is_s1 = s1.contains(&ramo_key);
        if !is_s1 {
            // no es elegible bajo la política estricta (solo S1)
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
        let mut dias_set: HashSet<String> = HashSet::new();

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
        // si pertenece a S1, añadir nodo para semestre 1 (guardando ramo_key)
        if is_s1 {
            let n = graph.add_node(node_meta.len());
            node_meta.push((idx, 1u8, ramo_key.clone()));
            priorities.insert(n, prioridad);
        }
    }

    // Conectar nodos: secciones compatibles y sin conflicto horario.
    // node_meta índice corresponde al payload asociado al node index (node.index()).
    for a in 0..node_meta.len() {
        for b in (a + 1)..node_meta.len() {
            let (sec_a_idx, _sem_a, _ramo_a) = &node_meta[a];
            let (sec_b_idx, _sem_b, _ramo_b) = &node_meta[b];
            let sec_a = &filtered[*sec_a_idx];
            let sec_b = &filtered[*sec_b_idx];

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

    // Construir etiquetas legibles por nodo para logging (índice corresponde a NodeIndex.index())
    let mut node_labels: Vec<String> = Vec::new();
    for (seccion_idx, _sem, ramo_k) in node_meta.iter() {
        let s = &filtered[*seccion_idx];
        let label = format!("{} ({}) - Sección {}", &s.codigo[..std::cmp::min(7, s.codigo.len())], ramo_k, s.seccion);
        node_labels.push(label);
    }

    let mut prev_solutions: Vec<Vec<NodeIndex>> = Vec::new();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    eprintln!("📊 [get_clique_max_pond_with_prefs] Iniciando búsqueda de múltiples soluciones");
    eprintln!("   S1 elegibles: {} ramos", s1.len());
    eprintln!("   Grafo: {} nodos, {} aristas", graph.node_count(), graph.edge_count());

    let max_iterations = 8;

    let total_start = Instant::now();

    // GENERAR MÚLTIPLES SOLUCIONES: encontrar el clique máximo por backtracking limitado
    let mut max_clique: Vec<NodeIndex> = Vec::new();
    let mut current_clique: Vec<NodeIndex> = Vec::new();
    let mut visited = vec![false; graph.node_count()];
    
    // Leer configuración de logging (permite silenciar logs costosos en entornos de prueba)
    let enable_bt_logs = env::var("ENABLE_BACKTRACK_LOGS").unwrap_or("0".to_string()) == "1";
    let silent_ramos_env = env::var("SILENT_RAMOS").unwrap_or(String::new());
    let silent_ramos: HashSet<String> = silent_ramos_env
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Construir vector paralelo de ramo_keys por índice de nodo (útil para supresión selectiva de logs)
    let mut node_ramo_keys: Vec<String> = Vec::new();
    for (_sidx, _sem, ramo_k) in node_meta.iter() {
        node_ramo_keys.push(ramo_k.clone());
    }

    // Función de backtracking para encontrar cliques (con logging opcional y poda por cota)
    fn find_max_clique_backtrack(
        graph: &UnGraph<usize, ()>,
        current: &mut Vec<NodeIndex>,
        candidates: &[NodeIndex],
        max_found: &mut Vec<NodeIndex>,
        visited: &mut Vec<bool>,
        max_size: usize,
        node_labels: &[String],
        ramo_keys: &[String],
        enable_logs: bool,
        silent_ramos: &HashSet<String>,
    ) {
        // Poda por cota: si incluso tomando todos los candidatos restantes no superamos
        // el mejor encontrado, abortamos.
        if current.len() + candidates.len() <= max_found.len() {
            return;
        }

        if current.len() > max_found.len() && current.len() <= max_size {
            *max_found = current.clone();
            if enable_logs {
                eprintln!("BT: nuevo max_found (len={}): {:?}", max_found.len(), max_found.iter().map(|n| n.index()).collect::<Vec<_>>());
            }
        }
        if current.len() >= max_size {
            return; // No necesitamos más grandes que max_size
        }
        for (i, &node) in candidates.iter().enumerate() {
            if visited[node.index()] {
                continue;
            }

            // logging condicional: saltar si ramo está en lista silenciosa
            if enable_logs {
                let ramo = &ramo_keys[node.index()];
                if !silent_ramos.contains(ramo) {
                    eprintln!("BT: intentando nodo {} => {}", node.index(), node_labels[node.index()]);
                }
            }

            // Verificar si node es compatible con todos en current
            let mut compatible = true;
            for &existing in current.iter() {
                if !graph.contains_edge(node, existing) {
                    compatible = false;
                    break;
                }
            }
            if compatible {
                visited[node.index()] = true;
                current.push(node);
                if enable_logs {
                    let ramo = &ramo_keys[node.index()];
                    if !silent_ramos.contains(ramo) {
                        eprintln!("BT: añadir nodo {} => {} (current_len={})", node.index(), node_labels[node.index()], current.len());
                    }
                }
                // Recursar con los candidatos restantes
                find_max_clique_backtrack(
                    graph,
                    current,
                    &candidates[i + 1..],
                    max_found,
                    visited,
                    max_size,
                    node_labels,
                    ramo_keys,
                    enable_logs,
                    silent_ramos,
                );
                current.pop();
                visited[node.index()] = false;
                if enable_logs {
                    let ramo = &ramo_keys[node.index()];
                    if !silent_ramos.contains(ramo) {
                        eprintln!("BT: remover nodo {} => {} (current_len={})", node.index(), node_labels[node.index()], current.len());
                    }
                }
            }
        }
    }
    
    // Ordenar nodos por ID numérico (`numb_correlativo`) ascendente para explorar
    // en el orden de la malla (más simple/determinístico).
    let mut nodes: Vec<NodeIndex> = graph.node_indices().collect();
    nodes.sort_by_key(|&n| {
        // node_meta[node.index()] = (seccion_idx, semestre, ramo_key)
        let (_sidx, _sem, ramo_k) = &node_meta[n.index()];
        ramos_disponibles.get(ramo_k).map(|r| r.numb_correlativo).unwrap_or(0)
    });

    // Heurística rápida: intentar una clique greedy/ponderada antes del backtracking
    let greedy = find_max_weight_clique(&graph, &priorities);
    if greedy.len() > max_clique.len() {
        max_clique = greedy.clone();
    }

    // Ejecutar backtracking con poda y logging opcional. Pasamos `node_ramo_keys` y flags.
    find_max_clique_backtrack(
        &graph,
        &mut current_clique,
        &nodes,
        &mut max_clique,
        &mut visited,
        6,
        &node_labels,
        &node_ramo_keys,
        enable_bt_logs,
        &silent_ramos,
    );
    
    eprintln!("   Clique máximo encontrado: {} nodos", max_clique.len());
    
    // Ahora, generar soluciones variando removiendo nodos para diversidad
    let mut prev_solutions: Vec<Vec<NodeIndex>> = Vec::new();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
    // Primera solución: el clique máximo
    if max_clique.len() >= 2 {
        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();
        arr_aux_delete.sort_by_key(|&(_, prio)| prio);
        while arr_aux_delete.len() > 6 { arr_aux_delete.remove(0); }
        
        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        prev_solutions.push(solution_key);
        
        let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
        let total_score_i64: i64 = arr_aux_delete.len() as i64;
        
        for &(node_idx, prioridad) in &arr_aux_delete {
            let (seccion_idx, _sem, _ramo) = &node_meta[node_idx.index()];
            let seccion = filtered[*seccion_idx].clone();
            solution_entries.push((seccion, prioridad));
        }
        
        solutions.push((solution_entries, total_score_i64));
        eprintln!("      -> Solución 1 aceptada ({} cursos, score {})", arr_aux_delete.len(), total_score_i64);
    }
    
    // Generar variaciones removiendo el nodo de menor prioridad
    let mut graph_copy = graph.clone();
    for iter in 2..=10 {
        if max_clique.is_empty() { break; }
        // Remover el nodo de menor prioridad del clique máximo
        let mut clique_copy = max_clique.clone();
        clique_copy.sort_by_key(|&n| *priorities.get(&n).unwrap_or(&0));
        if let Some(last) = clique_copy.first().cloned() {
            graph_copy.remove_node(last);
            max_clique.retain(|&n| n != last);
        }
        
        if max_clique.len() < 2 { break; }
        
        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();
        arr_aux_delete.sort_by_key(|&(_, prio)| prio);
        while arr_aux_delete.len() > 6 { arr_aux_delete.remove(0); }
        
        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        if prev_solutions.contains(&solution_key) { continue; }
        prev_solutions.push(solution_key);
        
        let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
        let total_score_i64: i64 = arr_aux_delete.len() as i64;
        
        for &(node_idx, prioridad) in &arr_aux_delete {
            let (seccion_idx, _sem, _ramo) = &node_meta[node_idx.index()];
            let seccion = filtered[*seccion_idx].clone();
            solution_entries.push((seccion, prioridad));
        }
        
        solutions.push((solution_entries, total_score_i64));
        eprintln!("      -> Solución {} aceptada ({} cursos, score {})", iter, arr_aux_delete.len(), total_score_i64);
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
