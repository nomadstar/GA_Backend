// Módulo de alto nivel para la ejecución de la Ruta Crítica
use std::error::Error;
use std::collections::HashMap;

// Declarar submódulos (archivos en la carpeta `src/algorithm`)
pub mod extract;
pub mod clique;
pub mod conflict;
pub mod pert;
pub mod ruta;

// Reexportar solo la API pública que quieres exponer desde aquí
pub use extract::{get_ramo_critico, extract_data};
pub use clique::{get_clique_with_user_prefs, get_clique_max_pond, find_max_weight_clique};
pub use conflict::horarios_tienen_conflicto;
pub use pert::set_values_recursive;

pub fn ejecutar_ruta_critica(
    params: Option<InputParams>,
) -> Result<Vec<(Vec<(crate::models::Seccion, i32)>, i64)>, Box<dyn std::error::Error>> {
    match params {
        Some(p) => ejecutar_ruta_critica_with_params(p),
        None => run_ruta_critica_solutions(),
    }
}

