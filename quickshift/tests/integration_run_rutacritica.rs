#[test]
fn integration_run_rutacritica_uses_pipeline() {
    // Llamamos a la API pública reexportada en `lib.rs` que orquesta la ruta crítica.
    // La función devolverá Err si algo falla en la lectura de ficheros u otros pasos.
    quickshift::run_ruta_critica().expect("run_ruta_critica should complete without error");
}
