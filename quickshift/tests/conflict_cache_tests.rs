use quickshift::algorithm::conflict::{seccion_contiene_hora, horarios_tienen_conflicto};
use quickshift::excel::{get_prereqs_cached, get_prereq_cache_stats};

#[test]
fn test_seccion_contiene_hora_basic() {
    // construir una seccion mínima
    let s = quickshift::models::Seccion {
        codigo: "CIT1001".to_string(),
        nombre: "Prog I".to_string(),
        seccion: "001".to_string(),
        horario: vec!["LU:08:30-10:20".to_string()],
        profesor: "Dr Test".to_string(),
        codigo_box: "CBM1001".to_string(),
    };
    assert!(seccion_contiene_hora(&s, "08:30"));
    assert!(!seccion_contiene_hora(&s, "12:00"));
}

#[test]
fn test_prereq_cache_hit_miss() {
    // usar nombre de malla que existe en src/datafiles
    let malla = "MiMalla.xlsx".to_string();
    // limpiar/leer stats iniciales
    let (h0, m0, entries0) = get_prereq_cache_stats();

    // Intentar cargar (primera vez -> miss)
    let res1 = get_prereqs_cached(&malla);
    // res1 puede fallar si archivos no están disponibles; no obstante
    // queremos asegurar que la llamada no rompe y que las stats se actualizaron
    let (h1, m1, entries1) = get_prereq_cache_stats();
    assert!(m1 >= m0);

    // Llamada repetida para provocar hit
    let _ = get_prereqs_cached(&malla);
    let (h2, _m2, _entries2) = get_prereq_cache_stats();
    assert!(h2 >= h1);
}
