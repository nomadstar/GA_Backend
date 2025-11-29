use quickshift::excel::mapeo::{MapeoMaestro, MapeoAsignatura};

#[test]
fn test_mapeo_basico() {
    let mut mapeo = MapeoMaestro::new();
    let mut asig = MapeoAsignatura::new("calculo i".to_string(), "Cálculo I".to_string());
    asig.id_malla = Some(6);
    asig.codigo_oa2024 = Some("CBM1001".to_string());
    asig.codigo_pa2025 = Some("CBM1001".to_string());
    asig.porcentaje_aprobacion = Some(68.77);
    mapeo.add_asignatura(asig);

    assert!(mapeo.get("calculo i").is_some());
    assert!(mapeo.get_by_codigo_oa("CBM1001").is_some());
    assert_eq!(mapeo.len(), 1);
}
