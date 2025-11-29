// Biblioteca raíz del crate `quickshift`.
// Reexporta los módulos principales y proporciona una función de conveniencia
// `run_ruta_critica` que orquesta el flujo principal.
pub mod excel;
pub mod algorithm;
pub mod models;
pub mod api_json;
pub mod server;
pub mod server_handlers;
pub mod analithics;

/// Ejecuta el servidor HTTP (reexport para facilitar uso desde `main`)
pub use server::run_server;

