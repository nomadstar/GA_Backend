// Biblioteca raíz del crate `quickshift`.
// Reexporta los módulos principales y proporciona una función de conveniencia
// `run_ruta_critica` que orquesta el flujo principal.
mod excel;
mod algorithm;
mod models;
mod api_json;
pub mod server;
pub mod analithics;

/// Ejecuta el servidor HTTP (reexport para facilitar uso desde `main`)
pub use server::run_server;

