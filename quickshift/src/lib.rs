// Biblioteca raíz del crate `quickshift`.
// Reexporta los módulos principales y proporciona una función de conveniencia
// `run_ruta_critica` que orquesta el flujo principal.
pub mod excel;
pub mod algorithms;
pub mod models;
pub mod rutacritica;
pub mod api_json;
pub mod rutacomoda;
pub mod server;

/// Ejecuta el flujo completo de Ruta Crítica (extracción -> procesamiento -> clique)
pub use rutacritica::run_ruta_critica;

/// Ejecuta el servidor HTTP (reexport para facilitar uso desde `main`)
pub use server::run_server;

