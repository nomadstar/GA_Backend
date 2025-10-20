// Algoritmos de ruta crítica y clique máximo

use std::collections::HashMap;
use petgraph::graph::{NodeIndex, DiGraph, UnGraph};
use petgraph::Direction;
use crate::models::{Seccion, RamoDisponible, PertNode};
use crate::excel::{leer_malla_excel, leer_oferta_academica_excel};

// Función de ruta crítica
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
        // Simular cálculo de camino más largo (simplificado)
        max_count_jump = std::cmp::max(max_count_jump, 2); // Simplificación
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

// Obtener ramos críticos
pub fn get_ramo_critico() -> (HashMap<String, RamoDisponible>, String, bool) {
    println!("Leyendo ramos críticos desde Excel...");
    
    let nombre_excel_malla = "MiMalla.xlsx";
    // Intentar leer porcentajes de aprobados (archivo adicional)
    let porcentajes_file = "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx";
    
    // Intentar leer desde Excel primero
    match leer_malla_excel(nombre_excel_malla) {
        Ok(ramos_disponibles) => {
            println!("✅ Datos leídos exitosamente desde {}", nombre_excel_malla);
            // Intentar cargar porcentajes y propagarlos a los ramos
            match crate::excel::leer_porcentajes_aprobados(porcentajes_file) {
                Ok(pc_map) => {
                    println!("📊 Datos de aprobados cargados: {} entradas", pc_map.len());
                    // Crear nueva mapa y asignar dificultad cuando exista: ID = A / n (fracción 0..1)
                    let mut new_map = ramos_disponibles.clone();
                    for (codigo, ramo) in new_map.iter_mut() {
                        // buscar por codigo o codigo_ref
                        if let Some(&(a, n)) = pc_map.get(codigo) {
                            if n > 0.0 {
                                ramo.dificultad = Some(a / n);
                            }
                        } else if let Some(ref cr) = ramo.codigo_ref {
                            if let Some(&(a, n)) = pc_map.get(cr) {
                                if n > 0.0 {
                                    ramo.dificultad = Some(a / n);
                                }
                            }
                        }
                    }

                    println!("Ramos críticos (dificultad ID = A/n):");
                    for (codigo, ramo) in &new_map {
                        if ramo.critico {
                            println!("->> {} - {} (ID: {:?})", ramo.nombre, codigo, ramo.dificultad.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "n/a".to_string()));
                        }
                    }

                    println!("\nRamos no críticos (ID mostrada):");
                    for (codigo, ramo) in &new_map {
                        if !ramo.critico {
                            println!("->> {} - {} (ID: {:?})", ramo.nombre, codigo, ramo.dificultad.map(|v| format!("{:.3}", v)).unwrap_or_else(|| "n/a".to_string()));
                        }
                    }

                    return (new_map, nombre_excel_malla.to_string(), true);
                }
                Err(e) => {
                    println!("⚠️ No se pudo leer porcentajes: {}. Procediendo sin dificultad.", e);
                }
            }
            
            println!("Ramos críticos:");
            for (codigo, ramo) in &ramos_disponibles {
                if ramo.critico {
                    println!("->> {} - {}", ramo.nombre, codigo);
                }
            }

            println!("\nRamos no críticos:");
            for (codigo, ramo) in &ramos_disponibles {
                if !ramo.critico {
                    println!("->> {} - {}", ramo.nombre, codigo);
                }
            }
            
            (ramos_disponibles, nombre_excel_malla.to_string(), true)
        }
        Err(e) => {
            println!("⚠️  No se pudo leer el archivo Excel: {}", e);
            println!("Usando datos de ejemplo...");
            
            // Fallback a datos simulados
            let (mapa, nombre) = create_fallback_data(nombre_excel_malla);
            (mapa, nombre, false)
        }
    }
}

// Extraer datos
pub fn extract_data(
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    _nombre_excel_malla: &str,
) -> (Vec<Seccion>, HashMap<String, RamoDisponible>, bool) {
    println!("Procesando extract_data...");
    
    // Use the Oferta Academica file bundled with the RutaCritica folder (path relative from quickshift/)
    let oferta_academica_file = "../RutaCritica/Oferta Academica 2021-1 vacantes 2021-02-04.xlsx";
    
    // Intentar leer oferta académica desde Excel
    match leer_oferta_academica_excel(oferta_academica_file) {
        Ok(mut lista_secciones) => {
            // Normalizar posibles valores inválidos en codigo_box (RUTs, horarios, 'X', números)
            for s in lista_secciones.iter_mut() {
                let cb = s.codigo_box.trim();
                let mut normalized = cb.to_string();

                // If codigo_box is empty, 'X' or looks like a RUT/number/hours (contains digits and dots or ':' or spaces), derive from s.codigo
                let looks_bad = normalized.is_empty()
                    || normalized == "X"
                    || normalized.chars().all(|c| c.is_numeric() || c == '.' || c == '-')
                    || normalized.chars().any(|c| c == ':' )
                    || normalized.split_whitespace().count() > 1 && normalized.chars().any(|c| c.is_numeric());

                if looks_bad {
                    // derive from s.codigo: take up to first space or '-' (e.g., "CBF1001 - CÁLCULO III" -> "CBF1001")
                    let derived = s.codigo.split(|c: char| c == ' ' || c == '-').next().unwrap_or(&s.codigo).to_string();
                    if !derived.is_empty() {
                        normalized = derived;
                    }
                }

                // Also remove stray dots and trim
                normalized = normalized.replace('.', "").trim().to_string();

                // assign back
                s.codigo_box = normalized;
            }

            // Filtrar solo las secciones que corresponden a ramos disponibles
            lista_secciones.retain(|seccion| {
                ramos_disponibles.contains_key(&seccion.codigo_box) ||
                ramos_disponibles.iter().any(|(_, ramo)| ramo.codigo == seccion.codigo_box)
            });
            
            println!("✅ Se encontraron {} secciones desde Excel", lista_secciones.len());
            (lista_secciones, ramos_disponibles.clone(), true)
        }
        Err(e) => {
            println!("⚠️  No se pudo leer oferta académica: {}", e);
            println!("Generando datos simulados...");
            
            // Fallback a datos simulados
            let (secs, map) = create_simulated_sections(ramos_disponibles);
            (secs, map, false)
        }
    }
}

// Verificar conflictos de horario
pub fn horarios_tienen_conflicto(horario1: &[String], horario2: &[String]) -> bool {
    for h1 in horario1 {
        for h2 in horario2 {
            if h1 == h2 {
                return true;
            }
        }
    }
    false
}

// Algoritmo de clique máximo
pub fn find_max_weight_clique(
    graph: &UnGraph<usize, ()>,
    priorities: &HashMap<NodeIndex, i32>,
) -> Vec<NodeIndex> {
    let nodes: Vec<_> = graph.node_indices().collect();
    
    // Algoritmo greedy mejorado: construir clique válido
    let mut sorted_nodes = nodes.clone();
    sorted_nodes.sort_by(|&a, &b| {
        priorities.get(&b).unwrap_or(&0).cmp(priorities.get(&a).unwrap_or(&0))
    });
    
    let mut current_clique = Vec::new();
    
    // Tomar el primer nodo (mayor prioridad)
    if let Some(&first_node) = sorted_nodes.first() {
        current_clique.push(first_node);
    }
    
    // Agregar nodos compatibles
    for &node in sorted_nodes.iter().skip(1) {
        // Verificar si el nodo es compatible con todos los nodos del clique actual
        let mut compatible = true;
        for &clique_node in &current_clique {
            if !graph.contains_edge(node, clique_node) {
                compatible = false;
                break;
            }
        }
        
        if compatible {
            current_clique.push(node);
            if current_clique.len() >= 6 { // Limitar a 6 ramos máximo
                break;
            }
        }
    }
    
    current_clique
}

// Generar clique máximo ponderado
pub fn get_clique_max_pond(
    lista_secciones: &Vec<Seccion>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    println!("=== Generador de Horarios ===");
    println!("Ramos disponibles:\n");
    
    for (i, (codigo, ramo)) in ramos_disponibles.iter().enumerate() {
        println!("{}.- {} || {}", i, ramo.nombre, codigo);
    }

    // Simular prioridades
    let mut priority_ramo: HashMap<String, i32> = HashMap::new();
    let mut priority_sec: HashMap<String, i32> = HashMap::new();
    
    // Ejemplo de prioridades predefinidas
    priority_ramo.insert("Algoritmos y Programación".to_string(), 90);
    priority_ramo.insert("Bases de Datos".to_string(), 85);
    priority_sec.insert("CIT3313-SEC1".to_string(), 95);

    // Construir grafo
    let mut graph = UnGraph::<usize, ()>::new_undirected();
    let mut node_indices = Vec::new();
    let mut priorities = HashMap::new();

    // Agregar nodos al grafo
    for (idx, seccion) in lista_secciones.iter().enumerate() {
        let ramo = &ramos_disponibles[&seccion.codigo_box];
        
        // Calcular prioridad según la lógica original
        let cc = if ramo.critico { 10 } else { 0 };
        let uu = 10 - ramo.holgura;
        let mut kk = 60 - ramo.numb_correlativo;
        
        // Aplicar prioridad de ramo si existe
        if let Some(&prio) = priority_ramo.get(&seccion.nombre) {
            kk = prio + 53;
        }
        
        let mut ss = seccion.seccion.parse::<i32>().unwrap_or(0);
        
        // Aplicar prioridad de sección si existe
        if let Some(&prio) = priority_sec.get(&seccion.codigo) {
            ss = prio + 20;
        }
        
        let prioridad = cc * 10000 + uu * 1000 + kk * 100 + ss;
        
        let node_idx = graph.add_node(idx);
        node_indices.push(node_idx);
        priorities.insert(node_idx, prioridad);
    }

    // Agregar aristas (conexiones entre secciones compatibles)
    for i in 0..node_indices.len() {
        for j in (i + 1)..node_indices.len() {
            let sec_i = &lista_secciones[graph[node_indices[i]]];
            let sec_j = &lista_secciones[graph[node_indices[j]]];
            
            // Verificar que no sean del mismo ramo y que no tengan conflictos de horario
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
    
    // Encontrar múltiples soluciones
    let mut prev_solutions = Vec::new();
    let mut graph_copy = graph.clone();
    let mut solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    for _solution_num in 1..=5 {
        let max_clique = find_max_weight_clique(&graph_copy, &priorities);

        if max_clique.len() <= 2 {
            println!("\n---------------");
            println!("Solo quedan soluciones con 2 o menos ramos");
            break;
        }

        let mut arr_aux_delete: Vec<(NodeIndex, i32)> = max_clique
            .iter()
            .map(|&idx| (idx, *priorities.get(&idx).unwrap_or(&0)))
            .collect();

        arr_aux_delete.sort_by_key(|&(_, prio)| prio);

        // Limitar a 6 ramos máximo
        while arr_aux_delete.len() > 6 {
            arr_aux_delete.remove(0);
        }

        // Verificar si ya se encontró esta solución
        let solution_key: Vec<_> = arr_aux_delete.iter().map(|&(idx, _)| idx).collect();
        if prev_solutions.contains(&solution_key) {
            // eliminar primer nodo y continuar
            if !arr_aux_delete.is_empty() {
                graph_copy.remove_node(arr_aux_delete[0].0);
            }
            continue;
        }

        println!("---------------");
        println!("\nSolución Recomendada :\n");

        // Construir la solución serializable
    let mut solution_entries: Vec<(Seccion, i32)> = Vec::new();
    let mut total_score_i64: i64 = 0;

        for &(node_idx, prioridad) in &arr_aux_delete {
            let seccion_idx = graph_copy[node_idx];
            let seccion = lista_secciones[seccion_idx].clone();
            let codigo_corto = &seccion.codigo[..std::cmp::min(7, seccion.codigo.len())];

            println!(
                "{} || {} - Sección: {} | Horario -> {:?} || {}",
                codigo_corto,
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

        // Remover un nodo para la siguiente iteración
        if !arr_aux_delete.is_empty() {
            graph_copy.remove_node(arr_aux_delete[0].0);
        }
    }

    solutions
}

// Funciones auxiliares privadas
fn create_fallback_data(nombre_excel_malla: &str) -> (HashMap<String, RamoDisponible>, String) {
    let mut ramos_disponibles = HashMap::new();
    
    ramos_disponibles.insert("CIT3313".to_string(), RamoDisponible {
        nombre: "Algoritmos y Programación".to_string(),
        codigo: "CIT3313".to_string(),
        holgura: 0,
        numb_correlativo: 53,
        critico: true,
        codigo_ref: Some("CIT3313".to_string()),
        dificultad: None,
    });
    
    ramos_disponibles.insert("CIT3211".to_string(), RamoDisponible {
        nombre: "Bases de Datos".to_string(),
        codigo: "CIT3211".to_string(),
        holgura: 0,
        numb_correlativo: 52,
        critico: true,
        codigo_ref: Some("CIT3211".to_string()),
        dificultad: None,
    });
    
    ramos_disponibles.insert("CIT3413".to_string(), RamoDisponible {
        nombre: "Redes de Computadores".to_string(),
        codigo: "CIT3413".to_string(),
        holgura: 2,
        numb_correlativo: 54,
        critico: false,
        codigo_ref: Some("CIT3413".to_string()),
        dificultad: None,
    });
    
    ramos_disponibles.insert("CFG-1".to_string(), RamoDisponible {
        nombre: "Curso de Formación General".to_string(),
        codigo: "CFG-1".to_string(),
        holgura: 3,
        numb_correlativo: 10,
        critico: false,
        codigo_ref: Some("CFG-1".to_string()),
        dificultad: None,
    });

    (ramos_disponibles, nombre_excel_malla.to_string())
}

fn create_simulated_sections(ramos_disponibles: &HashMap<String, RamoDisponible>) -> (Vec<Seccion>, HashMap<String, RamoDisponible>) {
    let mut lista_secciones = Vec::new();
    for (codigo_box, ramo) in ramos_disponibles {
        for seccion_num in 1..=2 {
            let horarios = match seccion_num {
                1 => vec!["LU 08:30".to_string(), "MI 08:30".to_string()],
                2 => vec!["MA 10:00".to_string(), "JU 10:00".to_string()],
                _ => vec!["VI 14:30".to_string()],
            };
            
            lista_secciones.push(Seccion {
                codigo: format!("{}-SEC{}", ramo.codigo, seccion_num),
                nombre: ramo.nombre.clone(),
                seccion: seccion_num.to_string(),
                horario: horarios,
                profesor: format!("Profesor {}", seccion_num),
                codigo_box: codigo_box.clone(),
            });
        }
    }
    
    (lista_secciones, ramos_disponibles.clone())
}