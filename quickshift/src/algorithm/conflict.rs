// Funciones para detectar conflictos de horario

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
