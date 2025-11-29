// Funciones para detectar conflictos de horario

pub fn horarios_tienen_conflicto(horario1: &[String], horario2: &[String]) -> bool {
    // Nuevo: parsear horarios y detectar solapamiento por día + rango de minutos
    fn parse_slots(h: &str) -> Vec<(String, i32, i32)> {
        // ejemplo: "LU MA JU 08:30-09:50" -> [(LU, 510, 590), (MA,510,590), ...]
        let s = h.trim().replace(".", ":").to_uppercase();
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() { return vec![]; }
        // buscar token que contiene '-'
        let mut time_token_idx: Option<usize> = None;
        for (i, &t) in parts.iter().enumerate() {
            if t.contains('-') {
                time_token_idx = Some(i);
                break;
            }
        }
        let time_idx = match time_token_idx {
            Some(i) => i,
            None => return vec![],
        };
        let time_tok = parts[time_idx];
        let times: Vec<&str> = time_tok.split('-').collect();
        if times.len() != 2 { return vec![]; }
        fn to_min(t: &str) -> Option<i32> {
            let tok = t.trim();
            let tok = if tok.len() == 4 && !tok.contains(':') {
                format!("0{}", tok)
            } else { tok.to_string() };
            let tok = tok.replace("AM", "").replace("PM", "").trim().to_string();
            let parts: Vec<&str> = tok.split(':').collect();
            if parts.len() != 2 { return None; }
            let hh = parts[0].parse::<i32>().ok()?;
            let mm = parts[1].parse::<i32>().ok()?;
            Some(hh * 60 + mm)
        }
        let start = to_min(times[0]).unwrap_or(0);
        let end = to_min(times[1]).unwrap_or(start + 60);
        let mut days = Vec::new();
        for d in &parts[..time_idx] {
            let dn = match d.chars().take(3).collect::<String>().as_str() {
                "LUN" | "L U" | "LU" => "LU",
                "MAR" | "MA" => "MA",
                "MIE" | "MI" => "MI",
                "JUE" | "JU" => "JU",
                "VIE" | "VI" => "VI",
                "SAB" | "SA" => "SA",
                "DOM" | "DO" => "DO",
                other => other,
            }.to_string();
            days.push(dn);
        }
        days.into_iter().map(|d| (d, start, end)).collect()
    }

    // parse all slots for both horarios
    let mut slots1: Vec<(String,i32,i32)> = Vec::new();
    for h in horario1 { slots1.extend(parse_slots(h)); }
    let mut slots2: Vec<(String,i32,i32)> = Vec::new();
    for h in horario2 { slots2.extend(parse_slots(h)); }

    for (d1, s1, e1) in slots1.iter() {
        for (d2, s2, e2) in slots2.iter() {
            if d1 == d2 {
                if s1 < e2 && s2 < e1 {
                    return true;
                }
            }
        }
    }
    false
}

pub fn horarios_violate_min_gap(horario1: &[String], horario2: &[String], min_minutes: i32) -> bool {
    // Devuelve true si en algún mismo día la distancia entre bloques es < min_minutes
    fn parse_slots(h: &str) -> Vec<(String, i32, i32)> {
        // reusar la misma lógica que arriba (podrías factorizar)
        let s = h.trim().replace(".", ":").to_uppercase();
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.is_empty() { return vec![]; }
        let mut time_token_idx: Option<usize> = None;
        for (i, &t) in parts.iter().enumerate() {
            if t.contains('-') { time_token_idx = Some(i); break; }
        }
        let time_idx = match time_token_idx { Some(i) => i, None => return vec![] };
        let time_tok = parts[time_idx];
        let times: Vec<&str> = time_tok.split('-').collect();
        if times.len() != 2 { return vec![]; }
        fn to_min(t: &str) -> Option<i32> {
            let tok = t.trim().replace("AM","").replace("PM","").to_string();
            let tok = if tok.len() == 4 && !tok.contains(':') { format!("0{}", tok) } else { tok };
            let parts: Vec<&str> = tok.split(':').collect();
            if parts.len() != 2 { return None; }
            let hh = parts[0].parse::<i32>().ok()?;
            let mm = parts[1].parse::<i32>().ok()?;
            Some(hh * 60 + mm)
        }
        let start = to_min(times[0]).unwrap_or(0);
        let end = to_min(times[1]).unwrap_or(start + 60);
        let mut days = Vec::new();
        for d in &parts[..time_idx] {
            let dn = match d.chars().take(3).collect::<String>().as_str() {
                "LUN" | "LU" => "LU",
                "MAR" | "MA" => "MA",
                "MIE" | "MI" => "MI",
                "JUE" | "JU" => "JU",
                "VIE" | "VI" => "VI",
                "SAB" | "SA" => "SA",
                "DOM" | "DO" => "DO",
                other => other,
            }.to_string();
            days.push(dn);
        }
        days.into_iter().map(|d| (d, start, end)).collect()
    }

    let mut slots1: Vec<(String,i32,i32)> = Vec::new();
    for h in horario1 { slots1.extend(parse_slots(h)); }
    let mut slots2: Vec<(String,i32,i32)> = Vec::new();
    for h in horario2 { slots2.extend(parse_slots(h)); }

    for (d1, s1, e1) in slots1.iter() {
        for (d2, s2, e2) in slots2.iter() {
            if d1 == d2 {
                // si hay solapamiento directo => viola claramente
                if s1 < e2 && s2 < e1 { return true; }
                // gap entre bloques
                let gap = if e1 <= s2 { s2 - e1 } else if e2 <= s1 { s1 - e2 } else { 0 };
                if gap < min_minutes { return true; }
            }
        }
    }
    false
}

use crate::models::Seccion;

/// Devuelve true si la sección contiene la hora indicada (ej: "08:30" o "8:30")
pub fn seccion_contiene_hora(seccion: &Seccion, hora_prohibida: &str) -> bool {
    // Reusar parser simple: comprobar si objetivo está dentro de algún bloque
    fn norm_time(t: &str) -> String {
        t.trim().replace(".", ":").to_uppercase()
    }
    let objetivo = norm_time(hora_prohibida);
    for h in seccion.horario.iter() {
        if norm_time(h).contains(&objetivo) || objetivo.contains(&norm_time(h)) {
            return true;
        }
    }
    false
}
