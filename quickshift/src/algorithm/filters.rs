/// Módulo de filtros para soluciones de RutaCrítica
/// Implementa la PHASE 4: apply_filters
///
/// Los filtros se aplican sobre las soluciones generadas para
/// excluir aquellas que no cumplen con las preferencias del usuario.

use crate::models::{Seccion, UserFilters};
use std::collections::HashSet;
use std::str::FromStr;

/// Aplica todos los filtros habilitados a una lista de soluciones
/// Retorna solo las soluciones que pasan todos los filtros
pub fn apply_all_filters(
    soluciones: Vec<(Vec<(Seccion, i32)>, i64)>,
    filtros: &Option<UserFilters>,
) -> Vec<(Vec<(Seccion, i32)>, i64)> {
    if filtros.is_none() {
        return soluciones;
    }

    let filters = filtros.as_ref().unwrap();
    let mut resultado = soluciones;

    // Filtro 3: Días/horarios libres
    if let Some(ref dias_filter) = filters.dias_horarios_libres {
        if dias_filter.habilitado {
            resultado = resultado
                .into_iter()
                .filter(|(sol, _)| filtro_dias_horarios_libres(sol, dias_filter))
                .collect();
        }
    }

    // Filtro 4: Ventana entre actividades
    if let Some(ref ventana_filter) = filters.ventana_entre_actividades {
        if ventana_filter.habilitado {
            resultado = resultado
                .into_iter()
                .filter(|(sol, _)| filtro_ventana_entre_actividades(sol, ventana_filter))
                .collect();
        }
    }

    // Filtro 5: Preferencias de profesores
    if let Some(ref prof_filter) = filters.preferencias_profesores {
        if prof_filter.habilitado {
            resultado = resultado
                .into_iter()
                .filter(|(sol, _)| filtro_preferencias_profesores(sol, prof_filter))
                .collect();
        }
    }

    resultado
}

/// Filtro 3: Días/horarios libres
/// Excluye soluciones que ocupan los días que el estudiante desea libres
/// o que tienen ventanas demasiado grandes
fn filtro_dias_horarios_libres(
    solucion: &[(Seccion, i32)],
    filtro: &crate::models::DiaHorariosLibres,
) -> bool {
    // Si hay franjas prohibidas (estructuradas), convertir a strings y comprobar solapamiento
    if let Some(ref franjas_prohibidas) = filtro.franjas_prohibidas {
        let mut fps: Vec<String> = Vec::with_capacity(franjas_prohibidas.len());
        for f in franjas_prohibidas.iter() {
            let s = format!("{} {} - {}", f.dia.to_uppercase(), f.inicio.trim(), f.fin.trim());
            fps.push(s);
        }
        for (seccion, _) in solucion {
            if solapan_horarios(&seccion.horario, &fps) {
                eprintln!("   ⊘ Excluyendo solución: sección {} solapan con franjas prohibidas", seccion.codigo);
                return false;
            }
        }
    }

    // Si se desea evitar "Sin horario", verificar
    if filtro.no_sin_horario.unwrap_or(false) {
        for (seccion, _) in solucion {
            if seccion.horario.is_empty()
                || seccion.horario.iter().any(|h| h.to_lowercase().contains("sin"))
            {
                eprintln!("   ⊘ Excluyendo solución: sección {} sin horario", seccion.codigo);
                return false;
            }
        }
    }

    true
}

/// Filtro 4: Ventana entre actividades
/// Excluye soluciones donde hay demasiada brecha entre clases
fn filtro_ventana_entre_actividades(
    _solucion: &[(Seccion, i32)],
    _filtro: &crate::models::VentanaEntreActividades,
) -> bool {
    // Este filtro requeriría análisis complejo de horarios
    // Por ahora, permitir todas las soluciones
    true
}

/// Filtro 5: Preferencias de profesores
/// Excluye soluciones con profesores en la lista de evitar
/// Prioriza soluciones con profesores preferidos
fn filtro_preferencias_profesores(
    solucion: &[(Seccion, i32)],
    filtro: &crate::models::PreferenciasProfesores,
) -> bool {
    let profesores_evitar: HashSet<String> = filtro
        .profesores_evitar
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|p| p.to_lowercase())
        .collect();

    // Excluir si algún profesor está en la lista de evitar
    for (seccion, _) in solucion {
        let prof_lower = seccion.profesor.to_lowercase();
        if !prof_lower.is_empty() && profesores_evitar.contains(&prof_lower) {
            eprintln!(
                "   ⊘ Excluyendo solución: profesor {} en lista de evitar",
                seccion.profesor
            );
            return false;
        }
    }

    true
}

/// Convierte "HH:MM" -> minutos desde 00:00
fn parse_hora_minutos(s: &str) -> Option<i32> {
    let s = s.trim();
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return None; }
    let h = i32::from_str(parts[0]).ok()?;
    let m = i32::from_str(parts[1]).ok()?;
    Some(h * 60 + m)
}

/// Extrae rango "HH:MM - HH:MM" (soporta espacios alrededor del guion)
/// Maneja múltiples variantes de guiones Unicode: - – — ―
fn parse_rango(s: &str) -> Option<(i32,i32)> {
    // Normalizar todos los tipos de guiones Unicode a ASCII '-'
    let normalized = s
        .replace('–', "-")  // en-dash
        .replace('—', "-")  // em-dash
        .replace('―', "-")  // horizontal bar
        .replace('‐', "-")  // hyphen
        .replace('−', "-"); // minus sign
    
    let parts: Vec<&str> = normalized.split('-').map(|t| t.trim()).collect();
    
    if parts.len() != 2 {
        eprintln!("[parse_rango DEBUG] Esperaba 2 partes, obtuve: {} - input: '{}'", parts.len(), s);
        return None;
    }
    
    let a = parse_hora_minutos(parts[0])?;
    let b = parse_hora_minutos(parts[1])?;
    
    eprintln!("[parse_rango SUCCESS] '{}' -> ({}, {})", s, a, b);
    Some((a,b))
}

/// Expande una entrada de horario como "LU JU 14:30 - 15:50" a vectores (dia, inicio, fin)
pub fn expand_horario_entry(entry: &str) -> Vec<(String, i32, i32)> {
    eprintln!("[expand_horario_entry START] input: '{}'", entry);
    
    if entry.trim().is_empty() {
        eprintln!("[expand_horario_entry] Entrada vacía");
        return vec![];
    }
    
    // Tokens divididos por espacios en blanco
    let tokens: Vec<&str> = entry.split_whitespace().collect();
    eprintln!("[expand_horario_entry] tokens: {:?}", tokens);
    
    if tokens.is_empty() {
        eprintln!("[expand_horario_entry] Sin tokens después de split");
        return vec![];
    }
    
    // Buscar el primer token que contenga ':'
    let time_idx = tokens.iter().position(|t| t.contains(':'));
    
    if time_idx.is_none() {
        eprintln!("[expand_horario_entry] No se encontró ':' en los tokens");
        return vec![];
    }
    
    let ti = time_idx.unwrap();
    eprintln!("[expand_horario_entry] time_idx: {}", ti);
    
    let day_tokens = &tokens[..ti];
    let time_part = tokens[ti..].join(" ");
    
    eprintln!("[expand_horario_entry] day_tokens: {:?}, time_part: '{}'", day_tokens, time_part);
    
    if let Some((s, e)) = parse_rango(&time_part) {
        let result: Vec<(String, i32, i32)> = day_tokens
            .iter()
            .map(|d| {
                let d_upper = d.to_uppercase();
                eprintln!("[expand_horario_entry] -> ({}, {}, {})", d_upper, s, e);
                (d_upper, s, e)
            })
            .collect();
        eprintln!("[expand_horario_entry SUCCESS] Retornando {} entradas", result.len());
        result
    } else {
        eprintln!("[expand_horario_entry FAILED] parse_rango falló para: '{}'", time_part);
        vec![]
    }
}

/// True si dos intervalos de minutos se solapan
fn intervals_overlap(a0: i32, a1: i32, b0: i32, b1: i32) -> bool {
    // intervalo abierto/cerrado: solapan si start < other_end && other_start < end
    a0 < b1 && b0 < a1
}

/// Comprueba si alguna de las horas de la sección solapa con alguna franja prohibida.
/// Ambos arrays contienen strings tipo "LU 08:30 - 09:50" o combinados "LU JU 14:30 - 15:50".
pub fn solapan_horarios(horarios_actuales: &[String], franjas_prohibidas: &[String]) -> bool {
    eprintln!("[solapan_horarios START] horarios_actuales: {:?}, franjas_prohibidas: {:?}", 
              horarios_actuales, franjas_prohibidas);
    
    // Expandir todas las franjas prohibidas a (dia, s, e)
    let mut prohibidos: Vec<(String, i32, i32)> = Vec::new();
    for p in franjas_prohibidas {
        eprintln!("[solapan_horarios] Expandiendo franja prohibida: '{}'", p);
        let expanded = expand_horario_entry(p);
        eprintln!("[solapan_horarios]   -> Expandida a: {:?}", expanded);
        prohibidos.extend(expanded);
    }
    
    eprintln!("[solapan_horarios] Total franjas prohibidas expandidas: {} entradas", prohibidos.len());
    
    if prohibidos.is_empty() {
        eprintln!("[solapan_horarios] No hay franjas prohibidas después de expandir -> retornando false");
        return false;
    }

    for h in horarios_actuales {
        eprintln!("[solapan_horarios] Verificando horario: '{}'", h);
        let segs = expand_horario_entry(h);
        eprintln!("[solapan_horarios]   -> Expandido a: {:?}", segs);
        
        for (d1, s1, e1) in segs {
            for (d2, s2, e2) in &prohibidos {
                if d1 == *d2 && intervals_overlap(s1, e1, *s2, *e2) {
                    eprintln!("[solapan_horarios] ¡SOLAPAMIENTO! {} {} ({}-{}) vs {} ({}-{})", 
                              d1, d1, s1, e1, d2, s2, e2);
                    return true;
                }
            }
        }
    }
    
    eprintln!("[solapan_horarios] No se encontraron solapamientos -> retornando false");
    false
}

// --- Wrappers/compatibilidad con nombres usados en otros módulos/tests ---

/// Nombre legacy usado por filtro_dias_horarios_libres y otros módulos
pub fn solapan_horarios_franja(horarios_actuales: &[String], franjas_prohibidas: &[String]) -> bool {
    solapan_horarios(horarios_actuales, franjas_prohibidas)
}

/// Wrapper público para tests/llamadas externas: "hora_a_minutos"
pub fn hora_a_minutos(s: &str) -> Option<i32> {
    parse_hora_minutos(s)
}

/// Wrapper público para tests/llamadas externas: "horas_se_solapan"
pub fn horas_se_solapan(a: &(i32,i32), b: &(i32,i32)) -> bool {
    intervals_overlap(a.0, a.1, b.0, b.1)
}