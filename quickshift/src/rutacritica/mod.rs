// Módulo de alto nivel para la ejecución de la Ruta Crítica

pub mod extract;
pub mod clique;
pub mod ruta;

/// Ejecuta el flujo completo de Ruta Crítica:
/// 1. Obtener ramos críticos (get_ramo_critico)
/// 2. Extraer datos de oferta (extract_data)
/// 3. Ejecutar clique máximo ponderado (get_clique_max_pond)
pub fn run_ruta_critica() -> Result<(), Box<dyn std::error::Error>> {
    println!("[rutacritica] Iniciando run_ruta_critica...");

    // 1) Obtener ramos críticos (devuelve mapa, nombre de archivo de malla y flag de lectura)
    let (ramos_disponibles, nombre_excel_malla, _malla_leida) = crate::algorithms::get_ramo_critico();

    println!(
        "[rutacritica] Ramos disponibles: {} entradas. Malla: {}",
        ramos_disponibles.len(), nombre_excel_malla
    );

    // 2) Extraer datos de secciones a partir de la oferta académica
    let (lista_secciones, _ramos_actualizados) =
        crate::rutacritica::extract::extract_data(ramos_disponibles.clone(), &nombre_excel_malla)?;

    println!(
        "[rutacritica] Secciones encontradas: {}",
        lista_secciones.len()
    );

    // 3) Ejecutar algoritmo de clique máximo ponderado (ahora devuelve soluciones)
    let soluciones = crate::algorithms::get_clique_max_pond(&lista_secciones, &ramos_disponibles);

    println!("[rutacritica] Soluciones obtenidas: {}", soluciones.len());

    println!("[rutacritica] run_ruta_critica finalizado.");
    Ok(())
}
