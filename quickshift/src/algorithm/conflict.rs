// Funciones para detectar conflictos y parsear franjas horarias.
use crate::models::Seccion;

fn to_min_opt(t: &str) -> Option<i32> {
    let mut tok = t.trim().to_uppercase().replace('.', ":");
    // quitar AM/PM si viene
    tok = tok.replace("AM", "").replace("PM", "").trim().to_string();
    if tok.len() == 4 && !tok.contains(':') { tok = format!("0{}", tok); }
    let parts: Vec<&str> = tok.split(':').collect();
    if parts.len() != 2 { return None; }
    let hh = parts[0].parse::<i32>().ok()?;
    let mm = parts[1].parse::<i32>().ok()?;
    // nota: no se ajusta AM/PM por simplicidad; si el usuario envía 08:00PM debería pasar como 20:00
    // pero como removemos AM/PM, asumimos formato 24h o correcto
    Some(hh * 60 + mm)
}

/// Parsear una cadena de horario a una lista de tuplas (DIA, start_min, end_min)
/// Ejemplo: "LU MA 08:30-10:00" -> [("LU",510,600),("MA",510,600)]
pub fn parse_slots(h: &str) -> Vec<(String, i32, i32)> {
    let s = h.trim().replace('.', ":").to_uppercase();
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.is_empty() { return vec![]; }
    let mut time_token_idx: Option<usize> = None;
    for (i, &t) in parts.iter().enumerate() {
        if t.contains('-') { time_token_idx = Some(i); break; }
    }
    let time_idx = match time_token_idx { Some(i) => i, None => return vec![] };
    let time_tok = parts[time_idx];
    // Manejar forma compacta "LU:08:30-10:00" donde el día y la hora están en el mismo token
    let mut days_prefix: Vec<String> = Vec::new();
    let mut actual_time_tok = time_tok;
    if time_idx == 0 && time_tok.contains(':') {
        // separar por la primera ':' para extraer posible día
        if let Some(pos) = time_tok.find(':') {
            let (maybe_day, rest) = time_tok.split_at(pos);
            // rest comienza con ':'; quitarla
            let rest = &rest[1..];
            // si la parte antes de ':' parece un día (2-3 letras), úsala
            let day_tok = maybe_day.trim();
            if !day_tok.is_empty() {
                days_prefix.push(day_tok.to_string());
                actual_time_tok = rest;
            }
        }
    }
    let times: Vec<&str> = actual_time_tok.split('-').collect();
    if times.len() != 2 { return vec![]; }
    let start = to_min_opt(times[0]).unwrap_or(0);
    let end = to_min_opt(times[1]).unwrap_or(start + 60);
    let mut days = Vec::new();
    // primero incluir cualquier prefijo de día extraído del mismo token (p.ej. "LU:08:30-...")
    for d in &days_prefix { days.push(d.clone()); }
    for d in &parts[..time_idx] {
        let token = d.trim().chars().take(3).collect::<String>();
        let dn = match token.as_str() {
            "LUN" | "LU" => "LU",
            "MAR" | "MA" => "MA",
            "MIE" | "MI" => "MI",
            "JUE" | "J U" | "JU" => "JU",
            "VIE" | "VI" => "VI",
            "SAB" | "SA" => "SA",
            "DOM" | "DO" => "DO",
            other => other,
        }.to_string();
        days.push(dn);
    }
    days.into_iter().map(|d| (d, start, end)).collect()
}

/// True si cualquiera de los slots de horario1 solapa con cualquiera de horario2 (mismo día y rango)
pub fn horarios_tienen_conflicto(horario1: &[String], horario2: &[String]) -> bool {
    let mut slots1: Vec<(String,i32,i32)> = Vec::new();
    for h in horario1 { slots1.extend(parse_slots(h)); }
    let mut slots2: Vec<(String,i32,i32)> = Vec::new();
    for h in horario2 { slots2.extend(parse_slots(h)); }
    for (d1, s1, e1) in slots1.iter() {
        for (d2, s2, e2) in slots2.iter() {
            if d1 == d2 {
                // Nuevo comportamiento: considerar conflicto sólo si la franja es exactamente la misma
                // (mismo inicio y fin). Permitimos solapamientos no exactos (varios ramos el mismo día)
                if s1 == s2 && e1 == e2 { return true; }
            }
        }
    }
    false
}

/// True si la distancia entre bloques en algún mismo día es < min_minutes (o hay solapamiento)
pub fn horarios_violate_min_gap(horario1: &[String], horario2: &[String], min_minutes: i32) -> bool {
    let mut slots1: Vec<(String,i32,i32)> = Vec::new();
    for h in horario1 { slots1.extend(parse_slots(h)); }
    let mut slots2: Vec<(String,i32,i32)> = Vec::new();
    for h in horario2 { slots2.extend(parse_slots(h)); }
    for (d1, s1, e1) in slots1.iter() {
        for (d2, s2, e2) in slots2.iter() {
            if d1 == d2 {
                if s1 < e2 && s2 < e1 { return true; }
                let gap = if *e1 <= *s2 { s2 - e1 } else if *e2 <= *s1 { s1 - e2 } else { 0 };
                if gap < min_minutes { return true; }
            }
        }
    }
    false
}

/// Comprueba si una sección contiene un tiempo (ej "08:30") dentro de alguno de sus bloques
pub fn seccion_contiene_hora(seccion: &Seccion, hora_prohibida: &str) -> bool {
    let objetivo_min = match to_min_opt(hora_prohibida) { Some(m) => m, None => return false };
    for h in seccion.horario.iter() {
        for (_d, s, e) in parse_slots(h) {
            if objetivo_min >= s && objetivo_min < e { return true; }
        }
    }
    false
}

/// True si la sección está completamente contenida en la franja `rango`.
/// `rango` puede contener días y una hora, p.ej. "LU 08:00-10:00" o "08:00-10:00".
pub fn seccion_contenida_en_rango(seccion: &Seccion, rango: &str) -> bool {
    let rango_slots = parse_slots(rango);
    if rango_slots.is_empty() { return false; }
    // Para cada slot de la sección, debe existir al menos un rango que contenga totalmente ese slot (mismo día)
    for h in seccion.horario.iter() {
        let seccion_slots = parse_slots(h);
        if seccion_slots.is_empty() { return false; }
        // Una sección puede tener múltiples días; consideramos que si alguno de sus slots NO está contenido -> fallamos
        for (d_s, s_s, e_s) in seccion_slots.iter() {
            let mut contained = false;
            for (d_r, s_r, e_r) in rango_slots.iter() {
                if d_r == d_s {
                    if s_s >= s_r && e_s <= e_r { contained = true; break; }
                }
            }
            if !contained { return false; }
        }
    }
    true
}
