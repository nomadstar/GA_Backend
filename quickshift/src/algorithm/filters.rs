/// Módulo de filtros para soluciones de RutaCrítica
/// Implementa la PHASE 4: apply_filters
///
/// Los filtros se aplican sobre las soluciones generadas para
/// excluir aquellas que no cumplen con las preferencias del usuario.

use crate::models::{Seccion, UserFilters};
use std::collections::HashSet;

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

    // Filtro 6: Balance entre líneas (no implementado aquí, requeriría mapeo de ramos a líneas)
    // if let Some(ref balance_filter) = filters.balance_lineas {
    //     if balance_filter.habilitado {
    //         resultado = resultado
    //             .into_iter()
    //             .filter(|(sol, _)| filtro_balance_lineas(sol, balance_filter))
    //             .collect();
    //     }
    // }

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
            if solapan_horarios(&seccion.horario, franjas_prohibidas) {
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

/// Detecta si algún horario en `horarios_actuales` solapan con alguno en `franjas_prohibidas`
/// Formato esperado: "LU 08:30-10:00", "MA VI 14:00-18:00", etc.
fn solapan_horarios(horarios_actuales: &[String], franjas_prohibidas: &[String]) -> bool {
    for horario_actual in horarios_actuales {
        for franja_prohibida in franjas_prohibidas {
            if horarios_se_solapan(horario_actual, franja_prohibida) {
                return true;
            }
        }
    }
    false
}

/// Compara dos horarios para detectar solapamiento
/// Formato: "LU MA JU 08:30-09:50" o "VI 14:30-15:50"
fn horarios_se_solapan(horario1: &str, horario2: &str) -> bool {
    // Extraer días y horas de ambos horarios
    let (dias1, horas1) = parse_horario(horario1);
    let (dias2, horas2) = parse_horario(horario2);

    // Si no comparten días, no hay solapamiento
    if !dias1.iter().any(|d| dias2.contains(d)) {
        return false;
    }

    // Si comparten días, verificar si las horas se solapan
    horas_se_solapan(&horas1, &horas2)
}

/// Parsea un horario en (conjunto de días, (hora_inicio, hora_fin))
/// Ejemplo: "LU MA JU 08:30-09:50" -> ({"LU", "MA", "JU"}, (830, 950))
fn parse_horario(horario: &str) -> (Vec<String>, (i32, i32)) {
    let partes: Vec<&str> = horario.split_whitespace().collect();

    // Último elemento debería ser el rango horario
    let horas_str = partes.last().unwrap_or(&"");
    let (inicio, fin) = if let Some((h1, h2)) = horas_str.split_once('-') {
        let h1_mins = hora_a_minutos(h1).unwrap_or(0);
        let h2_mins = hora_a_minutos(h2).unwrap_or(2400);
        (h1_mins, h2_mins)
    } else {
        (0, 2400)
    };

    // El resto son días
    let dias: Vec<String> = partes[0..partes.len().saturating_sub(1)]
        .iter()
        .map(|d| d.to_uppercase())
        .collect();

    (dias, (inicio, fin))
}

/// Convierte "HH:MM" a minutos desde medianoche
fn hora_a_minutos(hora: &str) -> Option<i32> {
    let partes: Vec<&str> = hora.split(':').collect();
    if partes.len() == 2 {
        let hh = partes[0].parse::<i32>().ok()?;
        let mm = partes[1].parse::<i32>().ok()?;
        Some(hh * 60 + mm)
    } else {
        None
    }
}

/// Verifica si dos rangos de horas se solapan
/// (inicio1, fin1) y (inicio2, fin2) en minutos
fn horas_se_solapan(h1: &(i32, i32), h2: &(i32, i32)) -> bool {
    !(h1.1 <= h2.0 || h2.1 <= h1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horas_se_solapan() {
        // 08:30-09:50 y 09:00-10:00 se solapan
        let h1 = (510, 590); // 08:30-09:50
        let h2 = (540, 600); // 09:00-10:00
        assert!(horas_se_solapan(&h1, &h2));

        // 08:00-09:00 y 09:00-10:00 no se solapan (límite)
        let h3 = (480, 540); // 08:00-09:00
        let h4 = (540, 600); // 09:00-10:00
        assert!(!horas_se_solapan(&h3, &h4));
    }

    #[test]
    fn test_hora_a_minutos() {
        assert_eq!(hora_a_minutos("08:30"), Some(510));
        assert_eq!(hora_a_minutos("14:00"), Some(840));
        assert_eq!(hora_a_minutos("23:59"), Some(1439));
    }
}
