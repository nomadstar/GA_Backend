use quickshift::algorithm::filters::{hora_a_minutos, horas_se_solapan};

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

#[test]
fn test_expand_horario_entry_supports_spaced_dash_and_multiple_days() {
    let slots = quickshift::algorithm::filters::expand_horario_entry("JU LU MA MI VI 08:30 - 09:50");
    assert_eq!(slots.len(), 5);
    assert_eq!(slots[0], ("JU".to_string(), 510, 590));
    assert_eq!(slots[1], ("LU".to_string(), 510, 590));
    assert_eq!(slots[2], ("MA".to_string(), 510, 590));
    assert_eq!(slots[3], ("MI".to_string(), 510, 590));
    assert_eq!(slots[4], ("VI".to_string(), 510, 590));
}

#[test]
fn test_solapan_horarios_detects_overlap_with_prohibited_slot() {
    let horarios_actuales = vec!["LU 08:30 - 09:50".to_string()];
    let franjas_prohibidas = vec!["LU 08:00 - 09:00".to_string()];
    assert!(quickshift::algorithm::filters::solapan_horarios(&horarios_actuales, &franjas_prohibidas));
}