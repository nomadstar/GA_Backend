// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica

pub fn ejecutar_ruta_critica() {
    println!("rutacritica::ruta -> ejecutar_ruta_critica");

    // Orquestador simple: lee la malla/oferta usando los helpers de `excel`
    // y ejecuta el planner de clique desde `algorithms`.
    let (ramos_disponibles, nombre_malla, malla_leida) = crate::algorithms::get_ramo_critico();
    println!("malla leida: {} -> {}", nombre_malla, malla_leida);

    let (lista_secciones, ramos_actualizados, oferta_leida) = crate::algorithms::extract_data(&ramos_disponibles, &nombre_malla);
    println!("secciones disponibles: {} (oferta leida: {})", lista_secciones.len(), oferta_leida);

    // Usamos el planner sin preferencias (InputParams vacío) para generar soluciones
    let params = crate::api_json::InputParams {
        email: String::new(),
        ramos_pasados: Vec::new(),
        ramos_prioritarios: Vec::new(),
        horarios_preferidos: Vec::new(),
        malla: None,
    };

    // Use the rutacritica wrapper which delegates to algorithms and
    // provides a stable integration point for route-critical analyses.
    crate::rutacritica::clique::run_clique(&lista_secciones, &ramos_actualizados);
}
