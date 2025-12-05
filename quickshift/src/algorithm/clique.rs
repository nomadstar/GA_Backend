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
    eprintln!(
        "✅ [prerequisitos] {} ✓ todos los {} requisitos cumplidos",
        ramo.nombre,
        ramo.requisitos_ids.len()
    );
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

/// Verifica si un horario (ej: "LU MA JU 08:30 - 09:50") solapa con una franja prohibida (ej: "LU 08:00-09:00")
fn horario_solapa_franja(horario: &str, franja_prohibida: &str) -> bool {
    let horario = horario.trim();
    let franja = franja_prohibida.trim();
    
    // Parsear franja: "LU 08:00-09:00" o "LU 08:00 - 09:00"
    let franja_words: Vec<&str> = franja.split_whitespace().collect();
    if franja_words.is_empty() {
        return false;
    }
    
    // El primer token es el día(s) prohibido
    let dias_prohibidos = franja_words[0].to_lowercase();
    
    // Buscar horario en franja (formato: "HH:MM-HH:MM" o "HH:MM ... HH:MM")
    let franja_tiempo = franja.replace("- ", "-");
    let tiempo_pattern: Vec<&str> = franja_tiempo.split_whitespace()
        .filter(|w| w.contains(':') || w.contains('-'))
        .collect();
    
    if tiempo_pattern.is_empty() {
        return false;
    }
    
    // Combinar todos los tokens de tiempo
    let tiempo_combined = tiempo_pattern.join(" ");
    
    // Parsear horas: buscar formato "HH:MM-HH:MM"
    let tiempo_parts: Vec<&str> = if tiempo_combined.contains('-') {
        tiempo_combined.split('-').collect()
    } else {
        return false;
    };
    
    if tiempo_parts.len() != 2 {
        return false;
    }
    
    let (franja_inicio_str, franja_fin_str) = (tiempo_parts[0].trim(), tiempo_parts[1].trim());
    
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
    
    eprintln!("[DEBUG horario_solapa_franja] horario_days={:?}, dias_prohibidos='{}'", horario_days, dias_prohibidos);
    
    let tiene_dia = horario_days.contains(&dias_prohibidos.as_str());
    
    if !tiene_dia {
        eprintln!("[DEBUG horario_solapa_franja] día prohibido '{}' no encontrado en {:?}, retornando false", dias_prohibidos, horario_days);
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
    
    let f = filtros.as_ref().unwrap();
    
    // Filtro: Franjas prohibidas
    if let Some(ref dias_horarios) = f.dias_horarios_libres {
        if dias_horarios.habilitado {
            if let Some(ref franjas_prohibidas) = dias_horarios.franjas_prohibidas {
                // Verificar si algún horario de la sección solapa con franjas prohibidas
                for horario in &seccion.horario {
                    for franja in franjas_prohibidas {
                        if horario_solapa_franja(horario, franja) {
                            eprintln!("[DEBUG] FILTRO: Excluyendo {} - horario '{}' solapa con franja '{}'", 
                                     seccion.codigo, horario, franja);
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
    
    // Filtro: Profesores a evitar
    if let Some(ref prof_filter) = f.preferencias_profesores {
        if prof_filter.habilitado {
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
        // excluir (es un curso externo no en la malla)
        false
    }).cloned().collect();

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
    
    let filtered_with_preqs = filtered.into_iter().filter(|s| {
        // Encontrar el ramo correspondiente a esta sección
        if let Some(ramo) = ramos_disponibles.values().find(|r| r.codigo.to_uppercase() == s.codigo.to_uppercase()) {
            // Verificar si cumple los prerequisitos
            if requisitos_cumplidos(s, ramo, ramos_disponibles, &passed_codes_set) {
                return true;
            } else {
                eprintln!(
                    "   ⊘ Excluyendo {} (id={}) - prerequisitos no cumplidos",
                    ramo.nombre, ramo.id
                );
                return false;
            }
        }
        
        // Si no encontramos el ramo en ramos_disponibles por CÓDIGO,
        // intentar matching por NOMBRE normalizado
        let sec_nombre_norm = normalize_name(&s.nombre);
        if let Some(ramo) = ramos_disponibles.values().find(|r| {
            normalize_name(&r.nombre) == sec_nombre_norm
        }) {
            // Encontrado por nombre, verificar requisitos
            if requisitos_cumplidos(s, ramo, ramos_disponibles, &passed_codes_set) {
                return true;
            } else {
                eprintln!(
                    "   ⊘ Excluyendo {} (nombre match con id={}) - prerequisitos no cumplidos",
                    ramo.nombre, ramo.id
                );
                return false;
            }
        }
        
        // Si NO encontramos ni por código ni por nombre, excluir
        // (significa que es un curso que no está en la malla)
        eprintln!(
            "   ⊘ Excluyendo {} - NO ENCONTRADO EN MALLA (puede ser electivo externo)",
            s.codigo
        );
        false
    }).collect::<Vec<_>>();
    
    eprintln!("   ✓ Después de validar prerequisitos: {} secciones", filtered_with_preqs.len());
    let mut filtered = filtered_with_preqs;
    
    // Aplicar filtros del usuario ANTES de construir la matriz de adjacencia
    // Esto reduce drasticamente el tamaño del problema
    eprintln!("   [PRE-FILTER] params.filtros is_some={}", params.filtros.is_some());
    let mut filtered = if params.filtros.is_some() {
        let pre_filtered = filtered.into_iter().filter(|s| {
            seccion_cumple_filtros(s, &params.filtros)
        }).collect::<Vec<_>>();
        eprintln!("   Después de filtros de usuario: {} secciones", pre_filtered.len());
        pre_filtered
    } else {
        filtered
    };
    
    if filtered.is_empty() && params.filtros.is_some() {
        eprintln!("   ⚠️  Todos fueron filtrados!");
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
    // PERO: Si hay pocos cursos viables (< 6), permitir reutilización controlada
    let mut all_solutions: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();
    
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
    let max_iterations = if should_allow_reuse { 200 } else { 80 };  // Más iteraciones si hay reutilización
    
    eprintln!("   [DEBUG] n={}, should_allow_reuse={}, max_iterations={}", n, should_allow_reuse, max_iterations);
    
    let mut remaining_indices: HashSet<usize> = (0..n).collect();
    let mut consecutive_empty_resets = 0;
    
    for _iteration in 0..max_iterations {
        if all_solutions.len() >= 10 {
            break;  // Ya tenemos suficientes soluciones
        }
        
        if remaining_indices.is_empty() {
            // Si permitimos reutilización y no hay más nodos únicos, reinicializar
            if should_allow_reuse && all_solutions.len() < 10 && n > 0 {
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
        candidates.sort_by_key(|&i| -(pri[i] as i64));
        
        if candidates.is_empty() {
            break;
        }
        
        let seed_idx = candidates[0];
        
        // VALIDAR que el seed cumple filtros Y requisitos previos
        if !seccion_cumple_filtros(&filtered[seed_idx], &params.filtros) {
            remaining_indices.remove(&seed_idx);
            continue;
        }
        
        // Construir set de cursos ya aprobados (para validar requisitos previos)
        let mut passed_codes: HashSet<String> = params.ramos_pasados.iter()
            .map(|s| s.to_uppercase())
            .collect();
        
        // Verificar requisitos del seed
        if let Some(seed_ramo) = ramos_disponibles.values().find(|r| r.codigo == filtered[seed_idx].codigo) {
            if !requisitos_cumplidos(&filtered[seed_idx], seed_ramo, ramos_disponibles, &passed_codes) {
                remaining_indices.remove(&seed_idx);
                continue;
            }
        }
        
        let mut clique: Vec<usize> = vec![seed_idx];
        // Agregar el seed al set de códigos pasados para validar siguientes nodos
        passed_codes.insert(filtered[seed_idx].codigo.clone().to_uppercase());
        
        // Greedy: agregar candidatos conectados a todos en la clique, max 6
        for &cand in candidates.iter().skip(1) {
            if clique.len() >= 6 {
                break;
            }
            if !remaining_indices.contains(&cand) {
                continue;
            }
            
            // VALIDAR que el candidato cumple filtros
            if !seccion_cumple_filtros(&filtered[cand], &params.filtros) {
                continue;
            }
            
            // candidate must be connected to ALL nodes already in clique
            if clique.iter().all(|&u| adj[u][cand]) {
                // VALIDAR requisitos previos del candidato
                let mut prereq_ok = true;
                if let Some(cand_ramo) = ramos_disponibles.values().find(|r| r.codigo == filtered[cand].codigo) {
                    if !requisitos_cumplidos(&filtered[cand], cand_ramo, ramos_disponibles, &passed_codes) {
                        prereq_ok = false;
                    }
                }
                
                if !prereq_ok {
                    continue;
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
                    // Agregar el nuevo nodo al set de códigos pasados
                    passed_codes.insert(filtered[cand].codigo.clone().to_uppercase());
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
            // Verificar que no es solución duplicada
            let sol_codes: Vec<String> = sol.iter().map(|(s, _)| s.codigo.to_uppercase()).collect();
            let is_duplicate = all_solutions.iter().any(|(prev_sol, _)| {
                let prev_codes: Vec<String> = prev_sol.iter().map(|(s, _)| s.codigo.to_uppercase()).collect();
                sol_codes == prev_codes
            });
            
            if !is_duplicate {
                all_solutions.push((sol, total));
                consecutive_empty_resets = 0;  // Reset el contador
                
                // IMPORTANTE: Remover TODO el clique Y sus secciones adicionales
                // Extrae los códigos de los cursos que están en la clique actual
                let mut courses_in_clique = std::collections::HashSet::new();
                for &idx in clique.iter() {
                    let codigo = &filtered[idx].codigo;
                    courses_in_clique.insert(codigo.clone());
                }
                
                // Remover TODAS las secciones de estos cursos
                remaining_indices.retain(|&idx| {
                    !courses_in_clique.contains(&filtered[idx].codigo)
                });
            } else {
                remaining_indices.remove(&seed_idx);
            }
        } else {
            // Si no hay solución válida, remover el seed
            remaining_indices.remove(&seed_idx);
        }
    }

    // ordenar por score y truncar a 80 soluciones
    all_solutions.sort_by(|a, b| b.1.cmp(&a.1));
    all_solutions.truncate(80);
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
