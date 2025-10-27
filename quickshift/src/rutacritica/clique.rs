// clique.rs - adaptador / wrapper para la lógica de clique

use crate::models::{Seccion, RamoDisponible};
use std::collections::HashMap;

/// Ejecuta get_clique_max_pond delegando en `crate::algorithms`.
pub fn run_clique(lista_secciones: &Vec<Seccion>, ramos_disponibles: &HashMap<String, RamoDisponible>) {
    println!("[rutacritica::clique] Ejecutando algoritmo de clique...");
    let soluciones = crate::algorithms::get_clique_max_pond(lista_secciones, ramos_disponibles);
    println!("[rutacritica::clique] soluciones: {}", soluciones.len());
}

/// Versión helper que construye datos de ejemplo y ejecuta el algoritmo.
pub fn run_clique_example() {
    // Usar la API pública que ya provee fallbacks internamente
    let (ramos_disponibles, nombre_malla, _malla_leida) = crate::algorithms::get_ramo_critico();
    let (lista_secciones, _, _oferta_leida) = crate::algorithms::extract_data(&ramos_disponibles, &nombre_malla);
    run_clique(&lista_secciones, &ramos_disponibles);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_clique_example_should_not_panic() {
        // The example runner uses fallback data when Excel files are missing.
        run_clique_example();
    }
}
