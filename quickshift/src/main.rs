// --- Sistema Generador de Horarios - Archivo principal ---

use quickshift::run_server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("=== Sistema Generador de Horarios (API) ===");
    let bind = "127.0.0.1:8080";
    println!("Iniciando servidor en http://{}", bind);
    run_server(bind).await
}
