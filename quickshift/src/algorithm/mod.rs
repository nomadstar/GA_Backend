// Módulo de alto nivel para la ejecución de la Ruta Crítica
use std::error::Error;
use std::collections::HashMap;
pub mod ruta;

pub fn ejecutar_ruta_critica(
    params: Option<InputParams>,
) -> Result<Vec<(Vec<(crate::models::Seccion, i32)>, i64)>, Box<dyn std::error::Error>> {
    match params {
        Some(p) => ejecutar_ruta_critica_with_params(p),
        None => run_ruta_critica_solutions(),
    }
}

