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

    let soluciones = crate::algorithms::get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    println!("--- Soluciones encontradas: {} ---", soluciones.len());
    for (i, (sol, score)) in soluciones.iter().enumerate() {
        println!("Solución #{} -> score: {} -> {} secciones", i + 1, score, sol.len());
        for (s, prio) in sol.iter() {
            println!(" - {} {} [{}] prioridad={}", s.codigo, s.nombre, s.seccion, prio);
        }
    }
}
