use quickshift::excel::normalize_codigo_nombre;

#[test]
fn detect_swap_nombre_id() {
    let nombre = "Álgebra y Geometría";
    let id = "1";
    let (codigo, nombre_out) = normalize_codigo_nombre(nombre, id);
    assert_eq!(codigo, "1");
    assert_eq!(nombre_out, "Álgebra y Geometría");
}

#[test]
fn keep_id_nombre() {
    let id = "7";
    let nombre = "Cálculo II";
    let (codigo, nombre_out) = normalize_codigo_nombre(id, nombre);
    assert_eq!(codigo, "7");
    assert_eq!(nombre_out, "Cálculo II");
}
