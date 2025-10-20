// --- Sistema Generador de Horarios - Archivo principal ---

mod models;
mod excel;
mod algorithms;
mod rutacomoda;

use algorithms::{get_ramo_critico, extract_data, get_clique_max_pond};
mod server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("=== Sistema Generador de Horarios (API) ===");
    let bind = "127.0.0.1:8080";
    println!("Iniciando servidor en http://{}", bind);
    server::run_server(bind).await
}
