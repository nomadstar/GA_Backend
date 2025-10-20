// Biblioteca raíz del crate `quickshift`.
// Reexporta los módulos principales y proporciona una función de conveniencia
// `run_ruta_critica` que orquesta el flujo principal.
pub mod excel;
pub mod algorithms;
pub mod models;
pub mod rutacritica;
pub mod api_json;

/// Ejecuta el flujo completo de Ruta Crítica (extracción -> procesamiento -> clique)
pub use rutacritica::run_ruta_critica;

