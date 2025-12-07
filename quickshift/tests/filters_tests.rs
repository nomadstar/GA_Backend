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