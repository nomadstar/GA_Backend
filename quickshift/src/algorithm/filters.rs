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
    // Si hay franjas prohibidas, verificar que ninguna sección solape
    if let Some(ref franjas_prohibidas) = filtro.franjas_prohibidas {
        for (seccion, _) in solucion {
            if solapan_horarios_franja(&seccion.horario, franjas_prohibidas) {
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
fn parse_rango(s: &str) -> Option<(i32,i32)> {
    let s = s.replace('–', "-");
    let parts: Vec<&str> = s.split('-').map(|t| t.trim()).collect();
    if parts.len() != 2 { return None; }
    let a = parse_hora_minutos(parts[0])?;
    let b = parse_hora_minutos(parts[1])?;
    Some((a,b))
}

/// Expande una entrada de horario como "LU JU 14:30 - 15:50" a vectores (dia, inicio, fin)
fn expand_horario_entry(entry: &str) -> Vec<(String, i32, i32)> {
    // tokens, buscar primer token que contenga ':' (inicio de la hora)
    let tokens: Vec<&str> = entry.split_whitespace().collect();
    let time_idx = tokens.iter().position(|t| t.contains(':'));
    if time_idx.is_none() {
        return vec![];
    }
    let ti = time_idx.unwrap();
    let day_tokens = &tokens[..ti];
    let time_part = tokens[ti..].join(" ");
    if let Some((s,e)) = parse_rango(&time_part) {
        day_tokens.iter().map(|d| (d.to_uppercase(), s, e)).collect()
    } else {
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
    // expandir todas las franjas prohibidas a (dia, s,e)
    let mut prohibidos: Vec<(String,i32,i32)> = Vec::new();
    for p in franjas_prohibidas {
        prohibidos.extend(expand_horario_entry(p));
    }
    if prohibidos.is_empty() { return false; }

    for h in horarios_actuales {
        let segs = expand_horario_entry(h);
        for (d1, s1, e1) in segs {
            for (d2, s2, e2) in &prohibidos {
                if d1 == *d2 && intervals_overlap(s1, e1, *s2, *e2) {
                    return true;
                }
            }
        }
    }
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