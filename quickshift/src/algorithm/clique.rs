/// clique.rs - Planificador minimalista: PERT + Cliques + Restricciones integradas
use std::collections::{HashMap, HashSet};
use petgraph::graph::{NodeIndex, UnGraph};
use crate::models::{Seccion, RamoDisponible};
use crate::excel::normalize_name;
use crate::api_json::InputParams;

/// Extrae hora en minutos desde inicio del día de un string "HH:MM"
fn parse_time_to_minutes(time_str: &str) -> Option<i32> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 2 { return None; }
    let hours = parts[0].trim().parse::<i32>().ok()?;
    let minutes = parts[1].trim().parse::<i32>().ok()?;
    Some(hours * 60 + minutes)
}

/// Extrae el rango de horas de un string como "LU MI 08:30 - 10:00" o "08:30-10:00"
fn parse_horario_range(horario: &str) -> Option<(i32, i32)> {
    // Normalizar guiones (reemplazar múltiples tipos de dash por "-")
    let normalized = horario
        .replace("–", "-") // en-dash
        .replace("—", "-") // em-dash
        .replace("−", "-") // minus sign
        .replace("‐", "-"); // hyphen
    
    // Buscar el patrón HH:MM-HH:MM o HH:MM - HH:MM
    // Primero encontramos las partes que contienen ":"
    let tokens: Vec<&str> = normalized.split_whitespace().collect();
    
    let mut start_time: Option<&str> = None;
    let mut end_time: Option<&str> = None;
    
    for (i, token) in tokens.iter().enumerate() {
        if token.contains(':') {
            // Este token tiene un tiempo
            if token.contains('-') {
                // Formato "08:30-10:00" todo junto
                let time_parts: Vec<&str> = token.split('-').collect();
                if time_parts.len() >= 2 {
                    start_time = Some(time_parts[0]);
                    end_time = Some(time_parts[1]);
                }
            } else if start_time.is_none() {
                start_time = Some(token);
            } else if end_time.is_none() {
                end_time = Some(token);
            }
        }
    }
    
    let start = parse_time_to_minutes(start_time?)?;
    let end = parse_time_to_minutes(end_time?)?;
    
    Some((start, end))
}

/// Extrae day symbols (LU, MA, MI, JU, VI) de un horario como "LU MA MI 08:30 - 10:00"
fn extract_days_from_horario(horario: &str) -> Vec<String> {
    let parts: Vec<&str> = horario.split_whitespace().collect();
    let mut days = Vec::new();
    
    for part in parts {
        let upper = part.to_uppercase();
        if matches!(upper.as_str(), "LU" | "MA" | "MI" | "JU" | "VI") {
            days.push(upper);
        }
    }
    
    days
}

/// Calcula el "compactness score" de una solución (0-100).
/// 
/// Una solución es más compacta si:
/// - Las clases se concentran en menos días
/// - Dentro de cada día, la duración (último horario - primer horario) es ≤ 5 horas
///
/// compactness_score = (compact_days / total_days_with_class) * 100
fn calculate_compactness_score(solution: &[(Seccion, i32)]) -> f64 {
    if solution.is_empty() { return 0.0; }
    
    // Mapear día a (start_min, end_min)
    let mut day_ranges: HashMap<String, (i32, i32)> = HashMap::new();
    
    for (seccion, _) in solution {
        for horario in &seccion.horario {
            let days = extract_days_from_horario(horario);
            if let Some((start, end)) = parse_horario_range(horario) {
                for day in days {
                    let entry = day_ranges.entry(day).or_insert((i32::MAX, 0));
                    entry.0 = entry.0.min(start);
                    entry.1 = entry.1.max(end);
                }
            }
        }
    }
    
    if day_ranges.is_empty() { return 0.0; }
    
    // Contar días compactos (duración ≤ 5 horas = 300 minutos)
    let compact_days = day_ranges.values()
        .filter(|(start, end)| end - start <= 300)
        .count() as f64;
    
    let total_days = day_ranges.len() as f64;
    (compact_days / total_days) * 100.0
}

/// Calcula total de gap/ventana entre clases en minutos para una solución.
/// 
/// Para cada día:
/// - Ordena horarios por hora inicio
/// - Suma los gaps entre horarios consecutivos
fn calculate_total_gaps(solution: &[(Seccion, i32)]) -> i32 {
    if solution.is_empty() { return 0; }
    
    // Mapear día a lista de (start, end) minutos
    let mut day_slots: HashMap<String, Vec<(i32, i32)>> = HashMap::new();
    
    for (seccion, _) in solution {
        for horario in &seccion.horario {
            let days = extract_days_from_horario(horario);
            if let Some((start, end)) = parse_horario_range(horario) {
                for day in days {
                    day_slots.entry(day)
                        .or_insert_with(Vec::new)
                        .push((start, end));
                }
            }
        }
    }
    
    let mut total_gaps = 0;
    
    for slots in day_slots.values_mut() {
        if slots.len() <= 1 { continue; }
        
        // Ordenar por start time
        slots.sort_by_key(|k| k.0);
        
        // Calcular gaps entre consecutivos
        for i in 0..slots.len()-1 {
            let gap = slots[i+1].0 - slots[i].1;
            if gap > 0 {
                total_gaps += gap;
            }
        }
    }
    
    total_gaps
}

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

/// Aplica modificadores de puntuación basados en optimizaciones seleccionadas
/// y ramos prioritarios del usuario.
/// 
/// PRIORIDADES (de mayor a menor peso):
/// 1. Ramos prioritarios: +100_000 por cada ramo prioritario en la solución
/// 2. Optimizaciones de días: ±10_000 * compactness
/// 3. Minimizar ventanas: -100 por minuto de ventana
/// 
/// Esto garantiza que los ramos prioritarios siempre tengan más peso que las ventanas.
fn apply_optimization_modifiers(base_score: i64, solution: &[(Seccion, i32)], params: &InputParams) -> i64 {
    let mut score = base_score;
    
    // DEBUG: siempre registrar que la función fue llamada
    let compactness = calculate_compactness_score(solution);
    let total_gaps = calculate_total_gaps(solution) as i64;
    
    // 1. BONUS POR RAMOS PRIORITARIOS (máxima prioridad)
    // +100_000 por cada ramo prioritario en la solución
    // Esto supera ampliamente cualquier penalización de ventanas (max ~12_000 para 2 horas)
    if !params.ramos_prioritarios.is_empty() {
        let priority_codes: std::collections::HashSet<String> = params.ramos_prioritarios
            .iter()
            .map(|s| normalize_name(s))
            .collect();
        
        let mut priority_count = 0;
        for (sec, _) in solution.iter() {
            let sec_code_norm = normalize_name(&sec.codigo);
            let sec_name_norm = normalize_name(&sec.nombre);
            
            if priority_codes.contains(&sec_code_norm) || priority_codes.contains(&sec_name_norm) {
                priority_count += 1;
            }
        }
        
        if priority_count > 0 {
            let priority_bonus = priority_count * 100_000i64;
            eprintln!("[OPT] ramos-prioritarios: {} ramos prioritarios, +{}", priority_count, priority_bonus);
            score += priority_bonus;
        }
    }
    
    // Solo mostrar debug si hay optimizaciones
    if !params.optimizations.is_empty() {
        eprintln!("[OPT-DEBUG] base_score={}, gaps={}min, compactness={:.2}%, opts={:?}", 
                  base_score, total_gaps, compactness, params.optimizations);
    }
    
    // 2. OPTIMIZACIONES DE HORARIO (menor prioridad que ramos prioritarios)
    for opt in &params.optimizations {
        eprintln!("[OPT-DEBUG] Processing optimization: {}", opt);
        match opt.as_str() {
            "compact-days" => {
                let modifier = (compactness as i64) * 10_000;
                eprintln!("[OPT] compact-days: +{}", modifier);
                score += modifier;
            }
            "spread-days" => {
                let modifier = (compactness as i64) * 10_000;
                eprintln!("[OPT] spread-days: -{}", modifier);
                score -= modifier;
            }
            "minimize-gaps" => {
                // Penalización por ventanas: -100 por minuto
                // Una ventana de 2 horas = -12_000, mucho menor que el bonus de 1 ramo prioritario (+100_000)
                let modifier = total_gaps * 100;
                eprintln!("[OPT] minimize-gaps: -{}", modifier);
                score -= modifier;
            }
            _ => {
                eprintln!("[OPT-DEBUG] Unknown optimization: {}", opt);
            }
        }
    }
    
    score
}

/// Verifica si los requisitos previos de una sección están cumplidos
/// Retorna true si:
/// - El curso NO tiene requisitos (requisitos_ids es vacío)
/// - El curso tiene requisitos Y TODOS ellos están en passed_codes
/// 
/// IMPORTANTE: Ahora soporta MÚLTIPLES prerequisitos.
/// Todos deben estar cumplidos para que el curso sea válido.
fn requisitos_cumplidos(
    _seccion: &Seccion,
    ramo: &RamoDisponible,
    ramos_disp: &HashMap<String, RamoDisponible>,
    passed_codes: &HashSet<String>,  // códigos de cursos ya pasados + cursos en solución actual
) -> bool {
    // Si no hay requisitos, está permitido
    if ramo.requisitos_ids.is_empty() {
        return true;
    }
    
    // Verificar que TODOS los requisitos están cumplidos
    for prereq_id in &ramo.requisitos_ids {
        // Buscar el ramo prerequisito por ID
        let prereq_ramo = match ramos_disp.values().find(|r| r.id == *prereq_id) {
            Some(r) => r,
            None => {
                eprintln!(
                    "⚠️  [prerequisitos] {} (id={}) requiere id={} pero no se encontró ese ramo",
                    ramo.nombre, ramo.id, prereq_id
                );
                return false;
            }
        };
        
        // Verificar si el código del prerequisito está en passed_codes
        let prereq_codigo_upper = prereq_ramo.codigo.to_uppercase();
        let cumplido = passed_codes.contains(&prereq_codigo_upper);
        
        if !cumplido {
            eprintln!(
                "❌ [prerequisitos] {} requiere: {} (id={}, código='{}')",
                ramo.nombre, prereq_ramo.nombre, prereq_ramo.id, prereq_ramo.codigo
            );
            return false;
        }
    }
    
    // Todos los requisitos están cumplidos
    // eprintln!(
    //     "✅ [prerequisitos] {} ✓ todos los {} requisitos cumplidos",
    //     ramo.nombre,
    //     ramo.requisitos_ids.len()
    // );
    true
}

/// Helper para parsear "HH:MM" a minutos
fn parse_hora(s: &str) -> Option<i32> {
    let s = s.trim();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    
    let h = parts[0].trim().parse::<i32>().ok()?;
    let m = parts[1].trim().parse::<i32>().ok()?;
    
    Some(h * 60 + m)
}

// Extrae rangos (día, inicio, fin) de un vector de horarios de sección
fn seccion_time_ranges(horarios: &Vec<String>) -> Vec<(String, i32, i32)> {
    let mut out = Vec::new();
    for h in horarios.iter() {
        // intentar parsear formato "LU MA JU 08:30 - 09:50"
        let horario = h.replace("- ", "-");
        // separar tokens
        let tokens: Vec<&str> = horario.split_whitespace().collect();
        if tokens.is_empty() { continue; }

        // buscar primer token que contiene ':' para identificar inicio tiempo
        let mut day_tokens: Vec<&str> = Vec::new();
        let mut time_tokens: Vec<&str> = Vec::new();
        for &t in tokens.iter() {
            if t.contains(":") || t.contains("-") {
                time_tokens.push(t);
            } else if time_tokens.is_empty() {
                day_tokens.push(t);
            }
        }

        if time_tokens.is_empty() || day_tokens.is_empty() { continue; }

        // join time tokens to find pattern like "08:30-09:50" or "08:30 - 09:50"
        let time_join = time_tokens.join(" ");
        let parts: Vec<&str> = if time_join.contains('-') { time_join.split('-').collect() } else { Vec::new() };
        if parts.len() != 2 { continue; }
        if let (Some(si), Some(sf)) = (parse_hora(parts[0].trim()), parse_hora(parts[1].trim())) {
            for &d in day_tokens.iter() {
                out.push((d.to_string().to_lowercase(), si, sf));
            }
        }
    }
    out
}

// Comprueba si dos secciones cumplen la ventana mínima entre clases (en minutos)
fn cumple_ventana_entre(se1: &Seccion, se2: &Seccion, minutos_min: i32) -> bool {
    let r1 = seccion_time_ranges(&se1.horario);
    let r2 = seccion_time_ranges(&se2.horario);
    for (d1, s1, e1) in r1.iter() {
        for (d2, s2, e2) in r2.iter() {
            if d1 == d2 {
                // desreferenciar valores numéricos (iter devuelve &i32 en tuples)
                let s1v = *s1; let e1v = *e1; let s2v = *s2; let e2v = *e2;
                // si se solapan la gap será 0; si no, calcular distancia mínima entre intervalos
                let gap = if e1v <= s2v { s2v - e1v } else if e2v <= s1v { s1v - e2v } else { 0 };
                if gap < minutos_min { return false; }
            }
        }
    }
    true
}

/// Verifica si un horario (ej: "LU MA JU 08:30 - 09:50") solapa con una franja prohibida (ej: "LU 08:00-09:00")
fn horario_solapa_franja(horario: &str, franja_prohibida: &crate::models::FranjaProhibida) -> bool {
    let horario = horario.trim();
    
    // Extraer día, inicio, fin de la estructura
    let dia_prohibido = franja_prohibida.dia.to_lowercase();
    let franja_inicio_str = &franja_prohibida.inicio;
    let franja_fin_str = &franja_prohibida.fin;
    
    // Parsear horas
    let franja_inicio = match parse_hora(franja_inicio_str) {
        Some(m) => m,
        None => {
            eprintln!("[DEBUG] No pude parsear hora inicio de franja: '{}'", franja_inicio_str);
            return false;
        }
    };
    
    let franja_fin = match parse_hora(franja_fin_str) {
        Some(m) => m,
        None => {
            eprintln!("[DEBUG] No pude parsear hora fin de franja: '{}'", franja_fin_str);
            return false;
        }
    };
    
    // Verificar que el día prohibido está en el horario
    // Los días están al inicio del horario (antes de las horas)
    // Formato: "LU MA JU 08:30 - 09:50" o "MI 14:30 - 15:50"
    let horario_lower = horario.to_lowercase();
    let horario_days: Vec<&str> = horario_lower.split_whitespace()
        .take_while(|w| !w.contains(':') && !w.contains('-'))
        .collect();
    
    eprintln!("[DEBUG horario_solapa_franja] horario_days={:?}, dia_prohibido='{}'", horario_days, dia_prohibido);
    
    let tiene_dia = horario_days.contains(&dia_prohibido.as_str());
    
    if !tiene_dia {
        eprintln!("[DEBUG horario_solapa_franja] día prohibido '{}' no encontrado en {:?}, retornando false", dia_prohibido, horario_days);
        return false; // Día no coincide
    }
    
    // Parsear horario: "LU MA JU 08:30 - 09:50" o "MI 14:30 - 15:50"
    let horario_tiempo = horario.replace("- ", "-");
    let horario_parts: Vec<&str> = horario_tiempo.split_whitespace()
        .filter(|w| w.contains(':') || w.contains('-'))
        .collect();
    
    if horario_parts.is_empty() {
        return false;
    }
    
    let horario_tiempo_combined = horario_parts.join(" ");
    
    let horario_tiempo_parts: Vec<&str> = if horario_tiempo_combined.contains('-') {
        horario_tiempo_combined.split('-').collect()
    } else {
        return false;
    };
    
    if horario_tiempo_parts.len() != 2 {
        return false;
    }
    
    let (horario_inicio_str, horario_fin_str) = (horario_tiempo_parts[0].trim(), horario_tiempo_parts[1].trim());
    
    let horario_inicio = match parse_hora(horario_inicio_str) {
        Some(m) => m,
        None => {
            eprintln!("[DEBUG] No pude parsear hora inicio de horario: '{}'", horario_inicio_str);
            return false;
        }
    };
    
    let horario_fin = match parse_hora(horario_fin_str) {
        Some(m) => m,
        None => {
            eprintln!("[DEBUG] No pude parsear hora fin de horario: '{}'", horario_fin_str);
            return false;
        }
    };
    
    // Verificar solapamiento temporal
    // Dos intervalos [a, b] y [c, d] solapan si a < d && c < b
    let solapa = franja_inicio < horario_fin && horario_inicio < franja_fin;
    
    if solapa {
        eprintln!("[DEBUG] SOLAPAMIENTO: franja=[{}-{}] horario=[{}-{}]", 
                 franja_inicio, franja_fin, horario_inicio, horario_fin);
    }
    
    solapa
}

/// Verifica si una sección cumple con los filtros del usuario
fn seccion_cumple_filtros(seccion: &Seccion, filtros: &Option<crate::models::UserFilters>) -> bool {
    if filtros.is_none() {
        return true;
    }
    
    // Las secciones CFG siempre pasan los filtros de usuario
    // (se tratan especialmente en la lógica de clique)
    if seccion.is_cfg {
        return true;
    }
    
    let f = filtros.as_ref().unwrap();
    
    // Filtro: Franjas prohibidas
    if let Some(ref dias_horarios) = f.dias_horarios_libres {
        if dias_horarios.habilitado {
            if let Some(ref franjas_prohibidas) = dias_horarios.franjas_prohibidas {
                // Verificar si algún horario de la sección solapa con franjas prohibidas
                for horario in &seccion.horario {
                    for franja in franjas_prohibidas {
                        if horario_solapa_franja(horario, franja) {
                            eprintln!("[DEBUG] FILTRO: Excluyendo {} - horario '{}' solapa con franja ({} {}:{})", 
                                     seccion.codigo, horario, franja.dia, franja.inicio, franja.fin);
                            return false;
                        }
                    }
                }
            }
            
            // Filtro: No sin horario
            if dias_horarios.no_sin_horario.unwrap_or(false) {
                if seccion.horario.is_empty() || 
                   seccion.horario.iter().any(|h| h.to_lowercase().contains("sin")) {
                    return false;
                }
            }
        }
    }
    
    // Filtro: Profesores a evitar / preferidos
    if let Some(ref prof_filter) = f.preferencias_profesores {
        if prof_filter.habilitado {
            // Si hay una lista de preferidos no vacía, requerir que el profesor esté en la lista
            if let Some(ref preferidos) = prof_filter.profesores_preferidos {
                if !preferidos.is_empty() {
                    let mut matched = false;
                    for pref in preferidos {
                        if seccion.profesor.to_lowercase().contains(&pref.to_lowercase()) {
                            matched = true; break;
                        }
                    }
                    if !matched { return false; }
                }
            }

            // Profesores a evitar siguen excluyendo
            if let Some(ref evitar) = prof_filter.profesores_evitar {
                for prof_evitar in evitar {
                    if seccion.profesor.to_lowercase().contains(&prof_evitar.to_lowercase()) {
                        return false;
                    }
                }
            }
        }
    }
    
    true
}

/// Búsqueda exhaustiva usando petgraph para máximas cliques
/// Prioriza CFGs y garantiza que aparezcan en soluciones
pub fn exhaustive_clique_search_with_cfg(
    filtered: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    max_size: usize,
    max_solutions: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    eprintln!("   [EXHAUSTIVE] Construyendo grafo de compatibilidad con petgraph...");
    
    // Construir grafo usando petgraph
    let mut graph: UnGraph<(usize, &Seccion), ()> = UnGraph::new_undirected();
    let mut node_map: HashMap<usize, NodeIndex> = HashMap::new();
    
    // Añadir nodos (secciones)
    for (idx, sec) in filtered.iter().enumerate() {
        let node_idx = graph.add_node((idx, sec));
        node_map.insert(idx, node_idx);
    }
    
    // Añadir aristas (compatibilidad entre secciones)
    for i in 0..filtered.len() {
        for j in (i + 1)..filtered.len() {
            let s1 = &filtered[i];
            let s2 = &filtered[j];
            
            // Verificar compatibilidad: mismo código? conflicto horario?
            let code_a = &s1.codigo[..std::cmp::min(7, s1.codigo.len())];
            let code_b = &s2.codigo[..std::cmp::min(7, s2.codigo.len())];
            
            let compatible = s1.codigo_box != s2.codigo_box 
                && code_a != code_b 
                && !sections_conflict(s1, s2)
                && seccion_cumple_filtros(s1, &params.filtros)
                && seccion_cumple_filtros(s2, &params.filtros);
            
            if compatible {
                if let (Some(&n1), Some(&n2)) = (node_map.get(&i), node_map.get(&j)) {
                    graph.add_edge(n1, n2, ());
                }
            }
        }
    }
    
    eprintln!("   [EXHAUSTIVE] Grafo: {} nodos, {} aristas", graph.node_count(), graph.edge_count());
    
    let mut all_solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut seen_solutions: HashSet<String> = HashSet::new();
    
    // Búsqueda exhaustiva de cliques usando DFS con backtracking
    fn find_cliques_dfs(
        node: NodeIndex,
        candidates: Vec<NodeIndex>,
        graph: &UnGraph<(usize, &Seccion), ()>,
        current_clique: &mut Vec<NodeIndex>,
        all_cliques: &mut Vec<Vec<NodeIndex>>,
        max_size: usize,
        max_cliques: usize,
        ramos_disponibles: &HashMap<String, RamoDisponible>,
    ) {
        if all_cliques.len() >= max_cliques {
            return;
        }
        
        // PODA: tamaño máximo alcanzado
        if current_clique.len() == max_size {
            all_cliques.push(current_clique.clone());
            return;
        }
        
        // PODA: no hay suficientes candidatos
        if current_clique.len() + candidates.len() < max_size {
            return;
        }
        
        // Base case: guardar cliques parciales válidos
        if candidates.is_empty() {
            if !current_clique.is_empty() {
                all_cliques.push(current_clique.clone());
            }
            return;
        }
        
        for (i, &cand) in candidates.iter().enumerate() {
            // Verificar que cand es compatible con todos en current_clique
            let mut compatible = true;
            for &existing in current_clique.iter() {
                if !graph.contains_edge(cand, existing) {
                    compatible = false;
                    break;
                }
            }
            
            if compatible {
                current_clique.push(cand);
                
                // Nuevos candidatos: solo aquellos conectados a cand
                let new_candidates: Vec<NodeIndex> = candidates[(i+1)..]
                    .iter()
                    .filter(|&&c| graph.contains_edge(cand, c))
                    .copied()
                    .collect();
                
                find_cliques_dfs(cand, new_candidates, graph, current_clique, 
                               all_cliques, max_size, max_cliques, ramos_disponibles);
                
                current_clique.pop();
            }
        }
    }
    
    // Ejecutar búsqueda comenzando desde cada nodo (priorizando CFGs)
    let mut all_nodes: Vec<NodeIndex> = graph.node_indices().collect();
    all_nodes.sort_by(|&a, &b| {
        let is_cfg_a = filtered[graph[a].0].is_cfg;
        let is_cfg_b = filtered[graph[b].0].is_cfg;
        b.index().cmp(&a.index())  // Orden determinista
    });
    
    let mut cliques_found: Vec<Vec<NodeIndex>> = Vec::new();
    
    for &start_node in &all_nodes {
        if cliques_found.len() >= max_solutions {
            break;
        }
        
        let neighbors: Vec<NodeIndex> = graph.neighbors(start_node).collect();
        let mut current = vec![start_node];
        find_cliques_dfs(start_node, neighbors, &graph, &mut current, 
                        &mut cliques_found, max_size, max_solutions, ramos_disponibles);
    }
    
    eprintln!("   [EXHAUSTIVE] Encontradas {} cliques", cliques_found.len());
    
    // Convertir cliques a soluciones
    for clique_nodes in cliques_found {
        let mut sol_vec: Vec<(Seccion, i32)> = Vec::new();
        let mut score = 0i64;
        
        for &node_idx in &clique_nodes {
            let (sec_idx, sec) = graph[node_idx];
            let priority = if let Some(r) = ramos_disponibles.values()
                .find(|r| r.codigo.to_uppercase() == sec.codigo.to_uppercase()) {
                compute_priority(r, sec) as i32
            } else if sec.is_cfg {
                10010150i32
            } else {
                0
            };
            
            sol_vec.push((sec.clone(), priority));
            score += priority as i64;
        }
        
        let key = sol_vec.iter()
            .map(|(s, _)| s.codigo_box.clone())
            .collect::<Vec<_>>()
            .join("|");
        
        if !seen_solutions.contains(&key) && !sol_vec.is_empty() {
            seen_solutions.insert(key);
            all_solutions.push((sol_vec, score));
        }
    }
    
    // Ordenar por score descendente
    all_solutions.sort_by(|a, b| b.1.cmp(&a.1));
    
    eprintln!("   [EXHAUSTIVE] ✅ {} soluciones únicamente después de deduplicación", all_solutions.len());
    all_solutions
}

pub fn get_clique_max_pond_with_prefs(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    // Implementación directa y concisa de "cliques reales" (greedy multi-seed).
    eprintln!("🧠 [clique] {} secciones, {} ramos", lista_secciones.len(), ramos_disponibles.len());
    
    let has_filters = params.filtros.is_some();
    eprintln!("   [DEBUG] has_filters={}, filtros={:?}", has_filters, 
              params.filtros.as_ref().map(|f| format!("UserFilters present")));

    // Calcular límite de CFGs: máximo 4 CFGs en total
    let cfgs_aprobados = params.ramos_pasados.iter()
        .filter(|r| r.to_uppercase().starts_with("CFG"))
        .count();
    let max_cfgs_permitidos = 4usize.saturating_sub(cfgs_aprobados);
    eprintln!("   [CFG-LIMIT] CFGs aprobados: {}, máximo permitido en soluciones: {}", 
              cfgs_aprobados, max_cfgs_permitidos);

    // --- Filtrado inicial (semestre y ramos pasados) ---
    let mut max_sem = 0;
    for code in &params.ramos_pasados {
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == *code) {
            if let Some(s) = r.semestre { max_sem = max_sem.max(s); }
        }
    }
    let max_sem = max_sem + 2;
    let passed: HashSet<_> = params.ramos_pasados.iter().cloned().collect();

    let mut filtered: Vec<Seccion> = lista_secciones.iter().filter(|s| {
        if passed.contains(&s.codigo) { return false; }  // Filtrar por código de curso, NO por codigo_box (package ID)
        
        // Intentar encontrar el ramo por CÓDIGO primero
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == s.codigo) {
            // Encontrado por código
            if let Some(sem) = r.semestre {
                return sem <= max_sem;
            } else {
                return true; // Sin semestre especificado, permitir
            }
        }
        
        // Si no encuentra por código, intentar por NOMBRE normalizado
        let sec_nombre_norm = normalize_name(&s.nombre);
        if let Some(r) = ramos_disponibles.values().find(|r| {
            normalize_name(&r.nombre) == sec_nombre_norm
        }) {
            // Encontrado por nombre
            if let Some(sem) = r.semestre {
                return sem <= max_sem;
            } else {
                return true; // Sin semestre especificado, permitir
            }
        }
        
        // Si NO encontramos en ramos_disponibles (ni por código ni por nombre),
        // permitir si es una sección CFG, si no excluir
        s.is_cfg
    }).cloned().collect();

    // Orden determinista de secciones para evitar no-determinismo por iteración
    filtered.sort_by(|a, b| {
        let ca = a.codigo.to_uppercase(); let cb = b.codigo.to_uppercase();
        let ord = ca.cmp(&cb);
        if ord != std::cmp::Ordering::Equal { ord } else { a.codigo_box.cmp(&b.codigo_box) }
    });
    eprintln!("   Filtrado: {} secciones", filtered.len());
    
    // ===============================================================
    // VALIDACIÓN DE PREREQUISITOS (filtrado crítico)
    // ===============================================================
    // Excluir cualquier curso cuyo prerequisito NO esté en ramos_pasados
    // Esto es OBLIGATORIO: no permitimos recomendar cursos sin prerequisitos cumplidos
    eprintln!("   [PREREQUISITOS] Filtrando secciones por requisitos previos...");
    eprintln!("   [DEBUG] Ramos con requisitos cargados:");
    
    let passed_codes_set: HashSet<String> = params.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    for ramo in ramos_disponibles.values().take(10) {
        if !ramo.requisitos_ids.is_empty() {
            eprintln!("     - {} (id={}) requiere: {:?}", ramo.nombre, ramo.id, ramo.requisitos_ids);
        }
    }
    
    // PYTHON-STYLE: Filtrado de prerequisitos MENOS estricto
    // En Python, solo se filtran prerequisitos para ELECTIVOS (importancia=2)
    // Los ramos normales NO se filtran por prerequisitos
    let filtered_with_preqs = filtered.into_iter().filter(|s| {
        // Encontrar el ramo correspondiente a esta sección
        if let Some(ramo) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == s.codigo.to_uppercase()) {
            // PYTHON-STYLE: Solo verificar prerequisitos para ELECTIVOS
            // Los ramos normales pasan sin verificación de prerequisitos
            if s.is_electivo {
                // Para electivos, verificar prerequisitos (como hace Python)
                if requisitos_cumplidos(s, ramo, ramos_disponibles, &passed_codes_set) {
                    return true;
                } else {
                    eprintln!(
                        "   ⊘ Excluyendo ELECTIVO {} (id={}) - prerequisitos no cumplidos",
                        ramo.nombre, ramo.id
                    );
                    return false;
                }
            } else {
                // Ramos normales: permitir SIN verificar prerequisitos (como Python)
                return true;
            }
        }
        
        // Si no encontramos el ramo en ramos_disponibles por CÓDIGO,
        // intentar matching por NOMBRE normalizado
        let sec_nombre_norm = normalize_name(&s.nombre);
        if let Some(ramo) = ramos_disponibles.values().find(|r| {
            normalize_name(&r.nombre) == sec_nombre_norm
        }) {
            // PYTHON-STYLE: Solo verificar prerequisitos para ELECTIVOS
            if s.is_electivo {
                if requisitos_cumplidos(s, ramo, ramos_disponibles, &passed_codes_set) {
                    return true;
                } else {
                    eprintln!(
                        "   ⊘ Excluyendo ELECTIVO {} (nombre match) - prerequisitos no cumplidos",
                        ramo.nombre
                    );
                    return false;
                }
            } else {
                return true;
            }
        }
        
        // Si NO encontramos ni por código ni por nombre,
        // permitir si la sección proviene de un CFG o es un electivo (lógica original)
        if s.is_cfg {
            eprintln!(
                "   ✓ Permitido {} - SECCIÓN CFG no encontrada en malla pero aceptada",
                s.codigo
            );
            return true;
        }
        
        if s.is_electivo {
            eprintln!(
                "   ✓ Permitido {} - ELECTIVO DE ESPECIALIZACIÓN no encontrado en malla pero aceptado",
                s.codigo
            );
            return true;
        }

        // Cursos no encontrados en malla: permitir (PYTHON-STYLE)
        eprintln!(
            "   ✓ Permitido {} - no encontrado en malla pero aceptado (PYTHON-STYLE)",
            s.codigo
        );
        true
    }).collect::<Vec<_>>();
    
    eprintln!("   ✓ Después de validar prerequisitos: {} secciones", filtered_with_preqs.len());
    let debug_cfg_count = filtered_with_preqs.iter().filter(|s| s.is_cfg).count();
    let debug_electivo_count = filtered_with_preqs.iter().filter(|s| s.is_electivo).count();
    eprintln!("   [DEBUG] Secciones CFG después de prerequisitos: {}", debug_cfg_count);
    eprintln!("   [DEBUG] Secciones ELECTIVOS después de prerequisitos: {}", debug_electivo_count);
    let mut filtered = filtered_with_preqs;
    
    // Aplicar filtros del usuario ANTES de construir la matriz de adjacencia
    // Esto reduce drasticamente el tamaño del problema
    eprintln!("   [PRE-FILTER] params.filtros is_some={}", params.filtros.is_some());
    let mut filtered = if params.filtros.is_some() {
        let pre_filtered = filtered.into_iter().filter(|s| {
            seccion_cumple_filtros(s, &params.filtros)
        }).collect::<Vec<_>>();
        eprintln!("   Después de filtros de usuario: {} secciones", pre_filtered.len());
        let debug_cfg_after = pre_filtered.iter().filter(|s| s.is_cfg).count();
        eprintln!("   [DEBUG] Secciones CFG después de filtros de usuario: {}", debug_cfg_after);
        pre_filtered
    } else {
        filtered
    };
    
    // FILTRO POR LÍMITE DE CFGs: Si el usuario ya completó su cuota de CFGs, eliminar todos los CFGs
    if max_cfgs_permitidos == 0 {
        eprintln!("   [CFG-FILTER] Usuario ya completó 4 CFGs - removiendo todos los CFGs del pool");
        filtered = filtered.into_iter().filter(|s| !s.is_cfg).collect();
        eprintln!("   Después de filtrar CFGs por límite: {} secciones", filtered.len());
    }
    
    if filtered.is_empty() && params.filtros.is_some() {
        eprintln!("   ⚠️  Todos fueron filtrados!");
        // FALLBACK: Si los filtros de usuario eliminaron TODAS las secciones,
        // retornar al menos una sección sin filtros de usuario para cumplir LEY FUNDAMENTAL
        eprintln!("   [FALLBACK LEY FUNDAMENTAL] Intentando retornar sin filtros de usuario...");
        
        // Revertir a las secciones antes de aplicar filtros de usuario
        let mut fallback_filtered: Vec<Seccion> = lista_secciones.iter().filter(|s| {
            if passed.contains(&s.codigo_box) { return false; }
            
            // Intentar encontrar el ramo por CÓDIGO primero
            if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == s.codigo) {
                if let Some(sem) = r.semestre {
                    return sem <= max_sem;
                } else { return true; }
            }
            let sec_nombre_norm = normalize_name(&s.nombre);
            if let Some(r) = ramos_disponibles.values().find(|r| normalize_name(&r.nombre) == sec_nombre_norm) {
                if let Some(sem) = r.semestre { return sem <= max_sem; } else { return true; }
            }
            false
        }).cloned().collect();

        // Filtrar solo secciones que cumplen prerequisitos
        let fallback_filtered: Vec<Seccion> = fallback_filtered.into_iter().filter(|s| {
            if let Some(r) = ramos_disponibles.values().find(|r| {
                if !r.codigo.is_empty() && !s.codigo.is_empty() {
                    return r.codigo == s.codigo;
                }
                normalize_name(&r.nombre) == normalize_name(&s.nombre)
            }) {
                let passed_codes_set: HashSet<String> = params.ramos_pasados.iter().map(|c| c.to_uppercase()).collect();
                return requisitos_cumplidos(s, r, ramos_disponibles, &passed_codes_set);
            }
            false
        }).collect();

        if !fallback_filtered.is_empty() {
            // Retornar la primer sección viable (mejor solución sin filtros)
            let s = &fallback_filtered[0];
            if let Some(r) = ramos_disponibles.values().find(|r| {
                if !r.codigo.is_empty() && !s.codigo.is_empty() {
                    if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
                }
                normalize_name(&r.nombre) == normalize_name(&s.nombre)
            }) {
                let score = compute_priority(r, s);
                let sol = vec![(s.clone(), score as i32)];
                let total = score;
                eprintln!("✅ [clique] 1 solución (fallback LEY FUNDAMENTAL - sin filtros de usuario)");
                return vec![(sol, total)];
            }
        }
    }

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
    
    // [DEBUG] Verificar conectividad de CFGs en el grafo
    let cfg_count = filtered.iter().filter(|s| s.is_cfg).count();
    if cfg_count > 0 {
        eprintln!("   [GRAPH-DEBUG] Verificando conectividad de {} CFGs en grafo de {} nodos", cfg_count, n);
        let mut cfgs_with_edges = 0;
        for i in 0..n {
            if filtered[i].is_cfg {
                let edge_count = adj[i].iter().filter(|&&connected| connected).count();
                if edge_count > 0 {
                    cfgs_with_edges += 1;
                }
                if edge_count == 0 {
                    eprintln!("      ⚠ CFG {} NO tiene edges (aislado)", filtered[i].codigo);
                }
            }
        }
        eprintln!("   [GRAPH-DEBUG] {}/{} CFGs tienen al menos 1 edge", cfgs_with_edges, cfg_count);
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
        let p = match candidate {
            Some(r) => compute_priority(r, s),
            None if s.is_cfg => {
                // CFG sin entrada en malla: asignar prioridad similar a cursos de 3er semestre
                eprintln!("   [DEBUG] CFG {} sin entrada en malla, asignando prioridad competitiva", s.codigo);
                10010150i64  // Similar a un curso no crítico, holgura media-baja, correlativo bajo
            },
            None if s.is_electivo => {
                // ELECTIVO DE CARRERA: prioridad más baja que obligatorios pero válida
                // Prioridad base: 00 05 30 00 (no crítico, holgura alta, correlativo medio)
                eprintln!("   [DEBUG] ELECTIVO {} sin entrada en malla, asignando prioridad de electivo", s.codigo);
                53000i64  // Prioridad más baja que cursos obligatorios pero mayor que 0
            },
            None => 0,
        };
        pri.push(p);
    }

    // --- Greedy multi-seed to build real cliques with max 6 courses ---
    // ESTRATEGIA OPTIMIZADA: Solo generar soluciones que MAXIMIZAN cursos (respetando PERT criticidad)
    // Si encontramos soluciones con 6 cursos -> guardar y seguir buscando DIFERENTES de 6
    // Detener cuando tengamos 10 soluciones con 6 cursos cada una
    let mut all_solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut cfg_selected_as_seed_count = 0;  // Contador de CFGs seleccionados como seed
    
    // FALLBACK para 1 sección: retornar como solución única (LEY FUNDAMENTAL)
    if n == 1 {
        eprintln!("   [DEBUG] Solo 1 sección viable. Retornando como solución única.");
        let s = filtered[0].clone();
        if let Some(r) = ramos_disponibles.values().find(|r| {
            if !r.codigo.is_empty() && !s.codigo.is_empty() {
                if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
            }
            normalize_name(&r.nombre) == normalize_name(&s.nombre)
        }) {
            let score = compute_priority(r, &s);
            let sol = vec![(s.clone(), score as i32)];
            let total = score;
            all_solutions.push((sol, total));
            eprintln!("✅ [clique] 1 solución (fallback para 1 sección viable)");
            return all_solutions;
        }
    }
    
    let should_allow_reuse = n < 6;  // Si hay menos de 6 secciones viables, permitir reutilización
    // OPTIMIZACIÓN PYTHON-STYLE: Más iteraciones para generar más soluciones
    // Con la estrategia de eliminar solo el nodo de menor prioridad, necesitamos más iteraciones
    // porque cada iteración solo elimina 1 nodo (vs todos los nodos de la solución)
    let max_iterations = if should_allow_reuse {
        1000usize  // Aumentado significativamente para permitir más variaciones
    } else {
        // Fórmula: queremos al menos n iteraciones para dar oportunidad a cada sección
        // Multiplicador alto porque solo eliminamos 1 nodo por solución encontrada
        let computed = std::cmp::max(500usize, n.saturating_mul(3));
        std::cmp::min(computed, 10000usize)  // Límite máximo aumentado
    };

    eprintln!("   [DEBUG] n={}, should_allow_reuse={}, max_iterations={} (PYTHON-STRATEGY)", n, should_allow_reuse, max_iterations);
    
    let mut remaining_indices: HashSet<usize> = (0..n).collect();
    let mut consecutive_empty_resets = 0;
    
    for _iteration in 0..max_iterations {
        // CAMBIO: Sin límites artificiales - generar TODAS las soluciones posibles
        // El límite se aplica solo por agotamiento del espacio de búsqueda o max_iterations
        
        if remaining_indices.is_empty() {
            // Si permitimos reutilización y no hay más nodos únicos, reinicializar
            if should_allow_reuse && all_solutions.len() < 15 && n > 0 {
                remaining_indices = (0..n).collect();
                consecutive_empty_resets += 1;
                
                // Si hemos reiniciado demasiadas veces, para evitar loop infinito
                if consecutive_empty_resets > 20 {
                    break;
                }
            } else {
                break;
            }
        }
        
        // Ordenar por prioridad dentro de índices restantes
        let mut candidates: Vec<usize> = remaining_indices.iter().copied().collect();
        // Orden determinista: primero por prioridad descendente, luego por índice ascendente
        candidates.sort_by(|&i, &j| pri[j].cmp(&pri[i]).then(i.cmp(&j)));
        
        if candidates.is_empty() {
            break;
        }
        
        let seed_idx = candidates[0];
        
        // [DEBUG] Track si el seed es CFG
        if filtered[seed_idx].is_cfg {
            cfg_selected_as_seed_count += 1;
            eprintln!("      [GREEDY-SEED] CFG {} seleccionado como seed (#{} vez)", filtered[seed_idx].codigo, cfg_selected_as_seed_count);
        }
        
        // VALIDAR que el seed cumple filtros Y requisitos previos
        if !seccion_cumple_filtros(&filtered[seed_idx], &params.filtros) {
            remaining_indices.remove(&seed_idx);
            continue;
        }
        
        // Construir set base de cursos ya aprobados (solo `ramos_pasados`) —STRICT: no permitimos
        // que la propia solución satisfaga prerequisitos (sin co-requisitos).
        let base_passed_codes: HashSet<String> = params.ramos_pasados.iter()
            .map(|s| s.to_uppercase())
            .collect();

        // PYTHON-STYLE: Solo verificar requisitos del seed si es ELECTIVO
        // Los CFGs no tienen prerequisitos, saltar validación (lógica original)
        // Los ramos normales tampoco verifican prerequisitos (como Python)
        if !filtered[seed_idx].is_cfg && filtered[seed_idx].is_electivo {
            if let Some(seed_ramo) = ramos_disponibles.values().find(|r| r.codigo == filtered[seed_idx].codigo) {
                if !requisitos_cumplidos(&filtered[seed_idx], seed_ramo, ramos_disponibles, &base_passed_codes) {
                    remaining_indices.remove(&seed_idx);
                    continue;
                }
            }
        }
        
        let mut clique: Vec<usize> = vec![seed_idx];
        
        // Greedy: agregar candidatos conectados a todos en la clique, max 6
        for &cand in candidates.iter().skip(1) {
            if clique.len() >= 6 {
                break;
            }
            if !remaining_indices.contains(&cand) {
                continue;
            }
            
            // VALIDAR límite de CFGs en el clique antes de agregar candidato
            let current_cfg_count = clique.iter().filter(|&&idx| filtered[idx].is_cfg && filtered[idx].codigo.to_uppercase().starts_with("CFG")).count();
            if filtered[cand].is_cfg && filtered[cand].codigo.to_uppercase().starts_with("CFG") {
                if current_cfg_count >= max_cfgs_permitidos {
                    continue;  // Ya alcanzamos el límite de CFGs
                }
            }
            
            // VALIDAR que el candidato cumple filtros
            if !seccion_cumple_filtros(&filtered[cand], &params.filtros) {
                continue;
            }
            
            // candidate must be connected to ALL nodes already in clique
                if clique.iter().all(|&u| adj[u][cand]) {
                    // No permitir el mismo curso dos veces dentro de una solución
                    let cand_code = filtered[cand].codigo.to_uppercase();
                    if clique.iter().any(|&u| filtered[u].codigo.to_uppercase() == cand_code) {
                        continue;
                    }
                // PYTHON-STYLE: Solo verificar requisitos para ELECTIVOS
                // Los ramos normales pasan sin verificación (como en Python)
                if filtered[cand].is_electivo && !filtered[cand].is_cfg {
                    let mut prereq_ok = true;
                    if let Some(cand_ramo) = ramos_disponibles.values().find(|r| r.codigo == filtered[cand].codigo) {
                        if !requisitos_cumplidos(&filtered[cand], cand_ramo, ramos_disponibles, &base_passed_codes) {
                            prereq_ok = false;
                        }
                    }
                    
                    if !prereq_ok {
                        continue;
                    }
                }
                
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
            
            // Los CFGs no están en ramos_disponibles, usar prioridad fija
            if s.is_cfg {
                let score = 10010150i64;  // Prioridad competitiva
                sol.push((s.clone(), score as i32));
                total += score;
            } else if let Some(r) = ramos_disponibles.values().find(|r| {
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
            // Verificar que no es solución duplicada (comparar por `codigo_box` de las secciones
            // para permitir variaciones de sección dentro del mismo ramo)
            let sol_section_keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            let mut sol_section_keys_sorted = sol_section_keys.clone();
            sol_section_keys_sorted.sort();
            let is_duplicate = all_solutions.iter().any(|(prev_sol, _)| {
                let mut prev_keys: Vec<String> = prev_sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
                prev_keys.sort();
                prev_keys == sol_section_keys_sorted
            });

            if !is_duplicate {
                // Aplicar modificadores de optimización ANTES de guardar
                let optimized_total = apply_optimization_modifiers(total, &sol, params);
                all_solutions.push((sol.clone(), optimized_total));
                consecutive_empty_resets = 0;  // Reset el contador
                
                // ESTRATEGIA PYTHON: Eliminar SOLO el nodo de menor prioridad de la solución
                // Esto permite generar más variaciones manteniendo los nodos de alta prioridad
                if !clique.is_empty() {
                    // Encontrar el índice con menor prioridad en el clique
                    let min_pri_idx = clique.iter()
                        .min_by_key(|&&idx| pri[idx])
                        .copied()
                        .unwrap_or(seed_idx);
                    remaining_indices.remove(&min_pri_idx);
                    eprintln!("   [PYTHON-STRATEGY] Removiendo nodo de menor prioridad: {} (pri={})", 
                              filtered[min_pri_idx].codigo, pri[min_pri_idx]);
                }
            } else {
                remaining_indices.remove(&seed_idx);
            }
        } else {
            // Si no hay solución válida, remover el seed
            remaining_indices.remove(&seed_idx);
        }
    }

    // Si la búsqueda greedy no produjo suficientes soluciones, usar el enumerador
    // exhaustivo como fallback para aumentar diversidad (hasta 15 soluciones para garantizar 10).
    eprintln!("   [GREEDY-SUMMARY] CFG seeds seleccionados: {}", cfg_selected_as_seed_count);
    
    if all_solutions.len() < 5 {
        eprintln!("   [FALLBACK] Solo {} soluciones desde greedy; ejecutando enumerador exhaustivo para aumentar diversidad...", all_solutions.len());
        // Generar combinaciones adicionales (limit aumentado para garantizar 10+)
        let mut extras = get_all_clique_combinations_with_pert(&filtered, ramos_disponibles, params, 6usize, 5000usize);
        // Mezclar sin duplicados (comparando por codigo_box ordenado)
        for (sol, total) in extras.drain(..) {
            let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            let mut is_dup = false;
            for (prev, _) in all_solutions.iter() {
                let mut prev_keys: Vec<String> = prev.iter().map(|(s, _)| s.codigo_box.clone()).collect();
                prev_keys.sort();
                if prev_keys.join("|") == key {
                    is_dup = true; break;
                }
            }
            if !is_dup {
                all_solutions.push((sol, total));
            }
            // CAMBIO: Sin límite artificial de 15
        }
        eprintln!("   [FALLBACK] now have {} solutions after merging extras", all_solutions.len());
    }

    // ordenar por score y aplicar estrategia de OPTIMIZACIÓN
    all_solutions.sort_by(|a, b| b.1.cmp(&a.1));
    
    // ESTRATEGIA DE FILTRADO INTELIGENTE:
    // SIN FILTROS: Solo retornar soluciones óptimas (máximo tamaño)
    // CON FILTROS: Permitir soluciones subóptimas si no hay suficientes óptimas
    let has_filters = params.filtros.is_some();
    let max_size = all_solutions.iter().map(|(sol, _)| sol.len()).max().unwrap_or(0);
    
    if !has_filters && max_size > 0 {
        // SIN FILTROS: Solo soluciones de tamaño máximo (determinista)
        // CAMBIO: Retornar TODAS las soluciones óptimas (sin límite de 20)
        let optimal: Vec<_> = all_solutions.into_iter().filter(|(sol, _)| sol.len() == max_size).collect();
        let optimal_count = optimal.len();
        
        all_solutions = optimal;
        eprintln!("✅ [clique] {} soluciones (max {} ramos, sin filtros = TODAS óptimas)", 
                  all_solutions.len(), max_size);
    } else {
        // CON FILTROS: Aplicar estrategia mixta (óptimas + subóptimas si es necesario)
        let has_six_course_solutions = all_solutions.iter().any(|(sol, _)| sol.len() == 6);
        if has_six_course_solutions {
            // Separar soluciones óptimas y subóptimas
            let optimal: Vec<_> = all_solutions.iter().cloned().filter(|(sol, _)| sol.len() == 6).collect();
            let mut suboptimal: Vec<_> = all_solutions.iter().cloned().filter(|(sol, _)| sol.len() != 6).collect();
            let optimal_count = optimal.len();
            
            // CAMBIO: Retornar TODAS las soluciones óptimas (sin límite artificial)
            let mut result = optimal;
            // Complementar con subóptimas para máxima diversidad
            suboptimal.sort_by(|a, b| b.1.cmp(&a.1));  // Ordenar subóptimas por score
            for (sol, score) in suboptimal {
                result.push((sol, score));
            }
            eprintln!("✅ [clique] {} soluciones TOTALES ({} óptimas + {} subóptimas)", 
                      result.len(), optimal_count, result.len() - optimal_count);
            all_solutions = result;
        } else {
            // Si no hay soluciones con 6 cursos, mantener TODAS
            eprintln!("✅ [clique] {} soluciones (max_weight_clique, max 6 ramos, sin 6-ramo solutions)", all_solutions.len());
        }
    }
    
    all_solutions
}

/// Wrapper público
pub fn get_clique_with_user_prefs(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    // DETERMINISMO + OPTIMALIDAD: Usar enumerador exhaustivo con límite MUY alto
    // para capturar TODAS las combinaciones válidas y retornar TOP 50
    let max_size = 6usize;
    let n_secciones = lista_secciones.len();
    
    // CAMBIO CRÍTICO: limit = 50,000 para garantizar captura de todas las cliques
    // Con 6 ramos × 20 secciones = 120 secciones, C(120,6) = 1.5B teórico
    // Pero filtrado por no-conflictos + 1 por ramo = ~5K-50K máximo realista
    let limit = 50_000usize;
    
    eprintln!("   [CLIQUE-DETERMINISM] secciones={}, limit={} (TOP 50 ENUMERATOR)", n_secciones, limit);
    eprintln!("   [GUARANTEE] Garantía: Enumeración exhaustiva retorna TOP 50 óptimos + subóptimos");
    
    let mut results = get_all_clique_combinations_with_pert(lista_secciones, ramos_disponibles, params, max_size, limit);
    
    // DETERMINISMO: Ordenar por score DESC, sin desempate (mostrar TODOS los empatados)
    // Esto permite ver múltiples soluciones con el mismo score
    results.sort_by(|a, b| b.1.cmp(&a.1)); // Score descendente (óptimos primero)
    
    // CAMBIO: Retornar TODAS las soluciones (sin truncar a 50)
    eprintln!("✅ [DETERMINISM] Retornando TODAS {} soluciones", results.len());
    results
}

/// Wrapper para generar más soluciones con un máximo de iteraciones personalizado
pub fn get_clique_max_pond_with_prefs_extended(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    max_iterations_override: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    // Simplemente reutilizar la función principal pero con más iteraciones
    // Modificar internamente el comportamiento del clique
    eprintln!("   [DEBUG] get_clique_max_pond_with_prefs_extended: max_iterations={}", max_iterations_override);
    
    // Por ahora, llamar a la función normal que ya usa dinámicamente las iteraciones
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

/// Backtracking enumerator que PRIORITIZA CFGs: garantiza que CFGs aparezcan en soluciones
fn enumerate_cliques_with_cfg_priority(
    filtered: &Vec<Seccion>,
    adj: &Vec<Vec<bool>>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    max_size: usize,
    limit: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let n = filtered.len();
    let mut results: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Precompute priorities
    let mut pri_cache: Vec<i64> = Vec::with_capacity(n);
    for s in filtered.iter() {
        let candidate = ramos_disponibles.values().find(|r| {
            if !r.codigo.is_empty() && !s.codigo.is_empty() {
                if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
            }
            normalize_name(&r.nombre) == normalize_name(&s.nombre)
        });
        let p = match candidate {
            Some(r) => compute_priority(r, s),
            None if s.is_cfg => 10010150i64,
            None if s.is_electivo => 53000i64,
            None => 0,
        };
        pri_cache.push(p);
    }

    // Separar CFGs y no-CFGs
    let cfg_indices: Vec<usize> = (0..n).filter(|&i| filtered[i].is_cfg).collect();
    let non_cfg_indices: Vec<usize> = (0..n).filter(|&i| !filtered[i].is_cfg).collect();
    
    eprintln!("   [CFG-PRIORITY] {} CFGs, {} no-CFGs", cfg_indices.len(), non_cfg_indices.len());

    // Estrategia 1: Empezar búsqueda desde CADA CFG como seed
    for &cfg_seed in &cfg_indices {
        if results.len() >= limit {
            break;
        }

        eprintln!("   [CFG-SEED] Partiendo de CFG en índice {} ({})", cfg_seed, filtered[cfg_seed].codigo);
        
        // Encuentra vecinos compatibles con este CFG
        let mut compatible: Vec<usize> = (0..n)
            .filter(|&i| i != cfg_seed && adj[cfg_seed][i])
            .collect();
        
        // Ordena vecinos por prioridad
        compatible.sort_by(|&a, &b| pri_cache[b].cmp(&pri_cache[a]));
        
        // Intenta construir cliques empezando con este CFG
        let mut current = vec![cfg_seed];
        
        // Greedy: agregar compatibles hasta llenar o sin más candidatos
        for &cand in &compatible {
            if current.len() >= max_size {
                break;
            }
            
            // Verificar que cand es compatible con TODOS en current
            if current.iter().all(|&u| adj[u][cand]) {
                // No permitir duplicado de código de curso
                let cand_code = filtered[cand].codigo.to_uppercase();
                if !current.iter().any(|&u| filtered[u].codigo.to_uppercase() == cand_code) {
                    current.push(cand);
                }
            }
        }

        // Construir solución
        let mut sol: Vec<(Seccion, i32)> = Vec::new();
        let mut total: i64 = 0;
        for &ix in &current {
            let s = filtered[ix].clone();
            let priority = if let Some(r) = ramos_disponibles.values()
                .find(|r| r.codigo.to_uppercase() == s.codigo.to_uppercase()) {
                compute_priority(r, &s) as i32
            } else if s.is_cfg {
                10010150i32
            } else {
                0
            };
            sol.push((s, priority));
            total += priority as i64;
        }

        // Aplicar optimizaciones
        let optimized_total = apply_optimization_modifiers(total, &sol, params);

        // Verificar duplicado
        let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
        keys.sort();
        let key = keys.join("|");
        
        if !seen.contains(&key) && !sol.is_empty() {
            seen.insert(key);
            results.push((sol, optimized_total));
        }
    }

    eprintln!("   [CFG-PRIORITY] {} soluciones generadas desde CFG seeds", results.len());
    results
}

/// Backtracking enumerator: genera combinaciones compatibles (cliques) hasta `max_size`.
/// - `limit` evita explosión combinatoria.
fn enumerate_clique_combinations(
    filtered: &Vec<Seccion>,
    adj: &Vec<Vec<bool>>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    max_size: usize,
    limit: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let n = filtered.len();
    let mut results: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Precompute candidate priorities to speed scoring
    let mut pri_cache: Vec<i64> = Vec::with_capacity(n);
    for s in filtered.iter() {
        let candidate = ramos_disponibles.values().find(|r| {
            if !r.codigo.is_empty() && !s.codigo.is_empty() {
                if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
            }
            normalize_name(&r.nombre) == normalize_name(&s.nombre)
        });
        let p = match candidate {
            Some(r) => compute_priority(r, s),
            None if s.is_cfg => {
                // CFG sin entrada en malla: asignar prioridad similar a cursos de 3er semestre
                10010150i64
            },
            None if s.is_electivo => {
                // ELECTIVO: prioridad más baja
                53000i64
            },
            None => 0,
        };
        pri_cache.push(p);
    }

    // Build an order vector of indices sorted by priority desc (tie: index asc)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| pri_cache[b].cmp(&pri_cache[a]).then(a.cmp(&b)));

    // Precompute prefix sums over pri ordered (for optimistic upper bound pruning)
    let mut pri_ordered: Vec<i64> = order.iter().map(|&i| pri_cache[i]).collect();
    let mut prefix: Vec<i64> = Vec::with_capacity(pri_ordered.len());
    let mut acc = 0i64;
    for &v in pri_ordered.iter() { acc += v; prefix.push(acc); }

    // Recursive backtracking with branch-and-bound using optimistic sum of top priorities
    fn dfs(
        start: usize,
        order: &Vec<usize>,
        filtered: &Vec<Seccion>,
        adj: &Vec<Vec<bool>>,
        ramos_disponibles: &HashMap<String, RamoDisponible>,
        params: &InputParams,
        max_size: usize,
        limit: usize,
        pri_cache: &Vec<i64>,
        prefix: &Vec<i64>,
        current: &mut Vec<usize>,
        current_total: i64,
        passed_codes: &mut HashSet<String>,
        results: &mut Vec<(Vec<(Seccion, i32)>, i64)>,
        seen: &mut HashSet<String>,
    ) {
        if results.len() >= limit { return; }

        // Record current (non-empty) solution
        if !current.is_empty() {
            // Use `codigo_box` (identificador de sección) so different sections of same course
            // are considered distinct solutions by the enumerator
            let mut keys: Vec<String> = current.iter().map(|&i| filtered[i].codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            if !seen.contains(&key) {
                let mut sol: Vec<(Seccion, i32)> = Vec::new();
                let mut total: i64 = 0;
                for &ix in current.iter() {
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
                    } else {
                        sol.push((s.clone(), 0));
                    }
                }
                // Aplicar modificadores de optimización
                let optimized_total = apply_optimization_modifiers(total, &sol, params);
                results.push((sol, optimized_total));
                seen.insert(key);
            }
        }

        if current.len() >= max_size { return; }

        // compute current minimum score among results (for pruning)
        let current_min_score = if results.len() < limit { i64::MIN } else { results.iter().map(|(_,s)| *s).min().unwrap_or(i64::MIN) };

        for pos in start..order.len() {
            if results.len() >= limit { break; }

            // optimistic upper bound: current_total + sum of next best (max_size - current.len()) pri
            let remaining_slots = max_size.saturating_sub(current.len());
            if remaining_slots > 0 {
                // we can take up to remaining_slots from prefix starting at pos
                let available = order.len().saturating_sub(pos);
                let take = std::cmp::min(remaining_slots, available);
                if take > 0 {
                    let sum_top = if pos == 0 { prefix[take-1] } else { prefix[pos+take-1] - prefix[pos-1] };
                    let optimistic = current_total + sum_top;
                    if results.len() >= limit && optimistic <= current_min_score {
                        // prune this branch
                        continue;
                    }
                }
            }

            let i = order[pos];

            // ensure compatibility with all in current
            let mut ok = true;
            for &u in current.iter() {
                if !adj[u][i] { ok = false; break; }
            }
            if !ok { continue; }

            // No permitir el mismo curso dos veces dentro de una solución (determinista)
            let i_code = filtered[i].codigo.to_uppercase();
            let mut already = false;
            for &u in current.iter() {
                if filtered[u].codigo.to_uppercase() == i_code { already = true; break; }
            }
            if already { continue; }

            // filters
            if !seccion_cumple_filtros(&filtered[i], &params.filtros) { continue; }

            if let Some(ref ventana) = params.filtros.as_ref().and_then(|f| f.ventana_entre_actividades.as_ref()) {
                if ventana.habilitado {
                    let minutos = ventana.minutos_entre_clases.unwrap_or(15);
                    let mut ventana_ok = true;
                    for &u in current.iter() {
                        if !cumple_ventana_entre(&filtered[u], &filtered[i], minutos) { ventana_ok = false; break; }
                    }
                    if !ventana_ok { continue; }
                }
            }

            // check prereqs STRICT: only `ramos_pasados` — no co-requisites allowed
            let local_passed: HashSet<String> = params.ramos_pasados.iter().map(|s| s.to_uppercase()).collect();

            if let Some(ramo_i) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == filtered[i].codigo.to_uppercase()) {
                if !requisitos_cumplidos(&filtered[i], ramo_i, ramos_disponibles, &local_passed) { continue; }
            } else {
                let sec_nombre_norm = normalize_name(&filtered[i].nombre);
                if let Some(ramo_i) = ramos_disponibles.values().find(|r| normalize_name(&r.nombre) == sec_nombre_norm) {
                    if !requisitos_cumplidos(&filtered[i], ramo_i, ramos_disponibles, &local_passed) { continue; }
                } else { continue; }
            }

            // include i (no se añade a `passed_codes`: no permitimos que un curso en la
            // misma solución sirva como prerequisito para otro)
            current.push(i);
            let added_score = pri_cache[i];

            // recurse next (pos+1 ensures combinations without reuse in ordered list)
            dfs(pos+1, order, filtered, adj, ramos_disponibles, params, max_size, limit, pri_cache, prefix, current, current_total + added_score, passed_codes, results, seen);

            // backtrack
            current.pop();

            if results.len() >= limit { break; }
        }
    }

    let mut current: Vec<usize> = Vec::new();
    let mut passed_codes: HashSet<String> = params.ramos_pasados.iter().map(|s| s.to_uppercase()).collect();
    
    eprintln!("🚀 [clique] Llamando a dfs con params.optimizations={:?}", params.optimizations);
    
    dfs(0, &order, filtered, adj, ramos_disponibles, params, max_size, limit, &pri_cache, &prefix, &mut current, 0, &mut passed_codes, &mut results, &mut seen);

    results
}

/// Enumerador con prioridad de tamaño: busca primero cliques del tamaño especificado
fn enumerate_clique_combinations_size_priority(
    filtered: &Vec<Seccion>,
    adj: &Vec<Vec<bool>>,
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    min_size: usize,
    max_size: usize,
    limit: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    let n = filtered.len();
    let mut results: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Precompute priorities
    let mut pri_cache: Vec<i64> = Vec::with_capacity(n);
    for s in filtered.iter() {
        let candidate = ramos_disponibles.values().find(|r| {
            if !r.codigo.is_empty() && !s.codigo.is_empty() {
                if r.codigo.to_lowercase() == s.codigo.to_lowercase() { return true; }
            }
            normalize_name(&r.nombre) == normalize_name(&s.nombre)
        });
        let p = match candidate {
            Some(r) => compute_priority(r, s),
            None if s.is_cfg => 10010150i64,
            None if s.is_electivo => 53000i64,
            None => 0,
        };
        pri_cache.push(p);
    }

    // Build order by priority
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| pri_cache[b].cmp(&pri_cache[a]).then(a.cmp(&b)));

    // Recursive DFS que PRIORIZA encontrar soluciones del tamaño objetivo
    fn dfs_size_priority(
        start: usize,
        order: &Vec<usize>,
        filtered: &Vec<Seccion>,
        adj: &Vec<Vec<bool>>,
        ramos_disponibles: &HashMap<String, RamoDisponible>,
        params: &InputParams,
        min_size: usize,
        max_size: usize,
        limit: usize,
        pri_cache: &Vec<i64>,
        current: &mut Vec<usize>,
        current_total: i64,
        results: &mut Vec<(Vec<(Seccion, i32)>, i64)>,
        seen: &mut HashSet<String>,
    ) {
        if results.len() >= limit { return; }

        // SOLO registrar si alcanzamos el tamaño mínimo
        if current.len() >= min_size {
            let mut keys: Vec<String> = current.iter().map(|&i| filtered[i].codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            
            if !seen.contains(&key) {
                let mut sol: Vec<(Seccion, i32)> = Vec::new();
                let mut total: i64 = 0;
                for &ix in current.iter() {
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
                    } else {
                        sol.push((s.clone(), 0));
                    }
                }
                let optimized_total = apply_optimization_modifiers(total, &sol, params);
                results.push((sol, optimized_total));
                seen.insert(key);
            }
        }

        if current.len() >= max_size { return; }

        for pos in start..order.len() {
            if results.len() >= limit { break; }

            let i = order[pos];

            // Compatibilidad
            let mut ok = true;
            for &u in current.iter() {
                if !adj[u][i] { ok = false; break; }
            }
            if !ok { continue; }

            // No duplicar curso
            let i_code = filtered[i].codigo.to_uppercase();
            let mut already = false;
            for &u in current.iter() {
                if filtered[u].codigo.to_uppercase() == i_code { already = true; break; }
            }
            if already { continue; }

            // Filtros
            if !seccion_cumple_filtros(&filtered[i], &params.filtros) { continue; }

            if let Some(ref ventana) = params.filtros.as_ref().and_then(|f| f.ventana_entre_actividades.as_ref()) {
                if ventana.habilitado {
                    let minutos = ventana.minutos_entre_clases.unwrap_or(15);
                    let mut ventana_ok = true;
                    for &u in current.iter() {
                        if !cumple_ventana_entre(&filtered[u], &filtered[i], minutos) { ventana_ok = false; break; }
                    }
                    if !ventana_ok { continue; }
                }
            }

            // Prerequisitos
            let local_passed: HashSet<String> = params.ramos_pasados.iter().map(|s| s.to_uppercase()).collect();
            if let Some(ramo_i) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == filtered[i].codigo.to_uppercase()) {
                if !requisitos_cumplidos(&filtered[i], ramo_i, ramos_disponibles, &local_passed) { continue; }
            } else {
                let sec_nombre_norm = normalize_name(&filtered[i].nombre);
                if let Some(ramo_i) = ramos_disponibles.values().find(|r| normalize_name(&r.nombre) == sec_nombre_norm) {
                    if !requisitos_cumplidos(&filtered[i], ramo_i, ramos_disponibles, &local_passed) { continue; }
                } else { continue; }
            }

            current.push(i);
            dfs_size_priority(pos+1, order, filtered, adj, ramos_disponibles, params, min_size, max_size, limit, pri_cache, current, current_total + pri_cache[i], results, seen);
            current.pop();

            if results.len() >= limit { break; }
        }
    }

    let mut current: Vec<usize> = Vec::new();
    dfs_size_priority(0, &order, filtered, adj, ramos_disponibles, params, min_size, max_size, limit, &pri_cache, &mut current, 0, &mut results, &mut seen);

    results
}

/// Genera todas (hasta un límite) las combinaciones compatibles y devuelve las mejores ordenadas por score.
pub fn get_all_clique_combinations_with_pert(
    lista_secciones: &[Seccion],
    ramos_disponibles: &HashMap<String, RamoDisponible>,
    params: &InputParams,
    max_size: usize,
    limit: usize,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    // Reuse initial filtering logic from get_clique_max_pond_with_prefs
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
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo == s.codigo) {
            if let Some(sem) = r.semestre { return sem <= max_sem; } else { return true; }
        }
        let sec_nombre_norm = normalize_name(&s.nombre);
        if let Some(r) = ramos_disponibles.values().find(|r| normalize_name(&r.nombre) == sec_nombre_norm) {
            if let Some(sem) = r.semestre { return sem <= max_sem; } else { return true; }
        }
        // Permitir CFG aunque no esté en malla
        s.is_cfg
    }).cloned().collect();

    let cfg_after_initial_filter = filtered.iter().filter(|s| s.is_cfg).count();
    eprintln!("   [ENUM] Después de filtrado inicial: {} secciones ({} CFGs)", filtered.len(), cfg_after_initial_filter);

    // --- SELLAR ramos que cumplen prerequisitos según ramos_pasados ---
    eprintln!("   [SEAL] Sellando ramos que cumplen prerequisitos con ramos_pasados...");
    let passed_codes_set: HashSet<String> = params.ramos_pasados.iter().map(|s| s.to_uppercase()).collect();

    // Map id -> codigo_upper for lookup
    let mut id_to_codigo: HashMap<i32, String> = HashMap::new();
    for r in ramos_disponibles.values() {
        id_to_codigo.insert(r.id, r.codigo.to_uppercase());
    }

    // Determinar ramos viables (sus prerequisitos todos están en passed_codes_set)
    let mut viable_ramo_ids: HashSet<i32> = HashSet::new();
    for r in ramos_disponibles.values() {
        if r.requisitos_ids.is_empty() {
            viable_ramo_ids.insert(r.id);
            continue;
        }
        let mut ok = true;
        for prereq_id in &r.requisitos_ids {
            if let Some(cod) = id_to_codigo.get(prereq_id) {
                if !passed_codes_set.contains(cod) {
                    ok = false; break;
                }
            } else {
                // prerequisito no encontrado -> no viable
                ok = false; break;
            }
        }
        if ok { viable_ramo_ids.insert(r.id); }
    }

    eprintln!("   [SEAL] ramos viables (según ramos_pasados): {} de {}", viable_ramo_ids.len(), ramos_disponibles.len());

    // Contar CFGs ANTES del filtrado SEAL
    let cfg_before_seal = filtered.iter().filter(|s| s.is_cfg).count();
    eprintln!("   [SEAL] CFGs antes de filtrado: {}", cfg_before_seal);

    // Filtrar secciones para dejar solo aquellas que pertenecen a ramos viables O son CFG
    let filtered: Vec<Seccion> = filtered.into_iter().filter(|s| {
        // Si es CFG, SIEMPRE permitir - no necesita estar en malla viable
        if s.is_cfg {
            eprintln!("   [SEAL-FILTER] ✓ Preservando CFG: {}", s.codigo);
            return true;
        }
        
        // Para no-CFG: verificar que pertenecen a ramos viables
        // match by codigo
        if let Some(r) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == s.codigo.to_uppercase()) {
            let viable = viable_ramo_ids.contains(&r.id);
            if !viable {
                eprintln!("   [SEAL-FILTER] ✗ Excluyendo no-CFG (no viable): {} (id={})", s.codigo, r.id);
            }
            return viable;
        }
        // match by normalized name
        let sec_nombre_norm = normalize_name(&s.nombre);
        if let Some(r) = ramos_disponibles.values().find(|r| normalize_name(&r.nombre) == sec_nombre_norm) {
            let viable = viable_ramo_ids.contains(&r.id);
            if !viable {
                eprintln!("   [SEAL-FILTER] ✗ Excluyendo no-CFG (no viable): {} (id={})", s.codigo, r.id);
            }
            return viable;
        }

        eprintln!("   [SEAL-FILTER] ✗ Excluyendo (no encontrado en malla): {}", s.codigo);
        false
    }).collect();

    eprintln!("   [SEAL] Después de sellar por prerequisitos: {} secciones", filtered.len());
    
    // Contar CFGs disponibles después del SEAL
    let cfg_count = filtered.iter().filter(|s| s.is_cfg).count();
    let non_cfg_count = filtered.len() - cfg_count;
    eprintln!("   [SEAL] {} CFG, {} no-CFG después de sellar", cfg_count, non_cfg_count);

    // build adjacency
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

    // Si hay CFGs disponibles, crear soluciones con CFGs como base
    let mut combos: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
    if cfg_count > 0 {
        eprintln!("   [CFG-PRIORITY] {} CFGs detectados - creando soluciones con CFGs", cfg_count);
        
        // Estrategia: Crear soluciones que incluyan cada CFG
        for (i, sec) in filtered.iter().enumerate() {
            if !sec.is_cfg {
                continue;
            }
            
            let cfg_priority = if let Some(r) = ramos_disponibles.values()
                .find(|r| r.codigo.to_uppercase() == sec.codigo.to_uppercase()) {
                compute_priority(r, sec) as i32
            } else {
                10010150i32
            };
            
            let mut sol = vec![(sec.clone(), cfg_priority)];
            let mut total = cfg_priority as i64;
            
            // Intentar agregar más secciones compatibles con este CFG
            for (j, other) in filtered.iter().enumerate() {
                if i == j || other.is_cfg { continue; }
                if sol.len() >= max_size { break; }
                if !adj[i][j] { continue; }
                
                // Evitar duplicados de código
                let other_code = other.codigo.to_uppercase();
                if sol.iter().any(|(s, _)| s.codigo.to_uppercase() == other_code) {
                    continue;
                }
                
                let other_priority = if let Some(r) = ramos_disponibles.values()
                    .find(|r| r.codigo.to_uppercase() == other.codigo.to_uppercase()) {
                    compute_priority(r, other) as i32
                } else {
                    0
                };
                
                sol.push((other.clone(), other_priority));
                total += other_priority as i64;
            }
            
            let optimized_total = apply_optimization_modifiers(total, &sol, params);
            
            // Verificar duplicado
            let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            
            let mut is_dup = false;
            for (prev, _) in combos.iter() {
                let mut prev_keys: Vec<String> = prev.iter().map(|(s, _)| s.codigo_box.clone()).collect();
                prev_keys.sort();
                if prev_keys.join("|") == key {
                    is_dup = true;
                    break;
                }
            }
            
            if !is_dup && !sol.is_empty() {
                combos.push((sol, optimized_total));
            }
            
            if combos.len() >= limit {
                break;
            }
        }
        
        eprintln!("   [CFG-PRIORITY] {} soluciones creadas desde CFGs", combos.len());
    }
    
    // Usar enumerador estándar para agregar más soluciones si es necesario
    if combos.len() < limit / 2 {
        eprintln!("   [STANDARD] Búsqueda exhaustiva estándar para diversidad...");
        let mut extras = enumerate_clique_combinations(&filtered, &adj, ramos_disponibles, params, max_size, limit);
        // Mezclar sin duplicados
        for (sol, score) in extras.drain(..) {
            let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            let mut is_dup = false;
            for (prev, _) in combos.iter() {
                let mut prev_keys: Vec<String> = prev.iter().map(|(s, _)| s.codigo_box.clone()).collect();
                prev_keys.sort();
                if prev_keys.join("|") == key {
                    is_dup = true;
                    break;
                }
            }
            if !is_dup {
                combos.push((sol, score));
            }
            if combos.len() >= limit { break; }
        }
    }

    // ===== ESTRATEGIA: Buscar PRIMERO todas las soluciones de 6 cursos =====
    eprintln!("   [SIZE-PRIORITY] Separando por tamaño y priorizando soluciones de 6 cursos");
    
    // Separar por tamaño
    let mut size_6: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut size_5: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    let mut size_other: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
    for (sol, score) in combos {
        match sol.len() {
            6 => size_6.push((sol, score)),
            5 => size_5.push((sol, score)),
            _ => size_other.push((sol, score)),
        }
    }
    
    eprintln!("   [SIZE-PRIORITY] {} soluciones de 6 cursos, {} de 5, {} otras", 
              size_6.len(), size_5.len(), size_other.len());
    
    // Si hay pocas soluciones de 6 cursos, buscar más exhaustivamente
    if size_6.len() < 50 {
        eprintln!("   [EXHAUSTIVE-6] Solo {} soluciones de 6 cursos - buscando más exhaustivamente", size_6.len());
        
        // Aumentar límite de búsqueda para encontrar MÁS soluciones de 6 cursos
        let extended_limit = 200_000usize;
        eprintln!("   [EXHAUSTIVE-6] Buscando con límite extendido: {}", extended_limit);
        
        let mut extended_combos = enumerate_clique_combinations_size_priority(
            &filtered, 
            &adj, 
            ramos_disponibles, 
            params, 
            6, // MIN_SIZE = 6
            6, // MAX_SIZE = 6  
            extended_limit
        );
        
        eprintln!("   [EXHAUSTIVE-6] Encontradas {} soluciones adicionales de 6 cursos", extended_combos.len());
        
        // Agregar las nuevas sin duplicados
        let mut seen_keys: HashSet<String> = HashSet::new();
        for (sol, _) in &size_6 {
            let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            keys.sort();
            seen_keys.insert(keys.join("|"));
        }
        
        for (sol, score) in extended_combos.drain(..) {
            let mut keys: Vec<String> = sol.iter().map(|(s, _)| s.codigo_box.clone()).collect();
            keys.sort();
            let key = keys.join("|");
            
            if !seen_keys.contains(&key) {
                seen_keys.insert(key);
                size_6.push((sol, score));
            }
        }
        
        eprintln!("   [EXHAUSTIVE-6] Total después de búsqueda extendida: {} soluciones de 6 cursos", size_6.len());
    }
    
    // Ordenar por score DESC
    size_6.sort_by(|a, b| b.1.cmp(&a.1));
    size_5.sort_by(|a, b| b.1.cmp(&a.1));
    size_other.sort_by(|a, b| b.1.cmp(&a.1));
    
    // PRIORIDAD: 6 cursos > 5 cursos > otros
    let mut final_combos: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
    // CAMBIO: Agregar TODAS las soluciones de 6 cursos (sin límite de 50)
    final_combos.extend_from_slice(&size_6);
    
    // Agregar TODAS las soluciones de 5 cursos
    if !size_5.is_empty() {
        final_combos.extend_from_slice(&size_5);
        eprintln!("   [SIZE-PRIORITY] Agregando {} soluciones de 5 cursos", size_5.len());
    }
    
    // Agregar TODAS las otras
    if !size_other.is_empty() {
        final_combos.extend_from_slice(&size_other);
        eprintln!("   [SIZE-PRIORITY] Agregando {} soluciones de otros tamaños", size_other.len());
    }
    
    eprintln!("   [ENUM-FINAL] Retornando {} combinaciones ({} de 6 cursos, {} otras)", 
              final_combos.len(), 
              final_combos.iter().filter(|(s, _)| s.len() == 6).count(),
              final_combos.iter().filter(|(s, _)| s.len() != 6).count());
    
    final_combos
}
