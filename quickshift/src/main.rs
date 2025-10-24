// --- Sistema Generador de Horarios - Archivo principal ---

use quickshift::run_server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("=== Sistema Generador de Horarios (API) ===");
    let bind = "127.0.0.1:8080"; // cambia la dirección por lo que consideres pertinente.
    println!("Iniciando servidor en http://{}", bind);
        println!("");
        println!("Endpoints disponibles:");
        println!("  POST /solve    - Body JSON. Ejemplo:");
        println!("{}", r#"{
    "email": "alumno@ejemplo.cl",
    "ramos_pasados": ["CIT3313", "CIT3211"],
    "ramos_prioritarios": ["CIT3313"],
    "horarios_preferidos": ["08:00-10:00"],
    "malla": "MallaCurricular2020.xlsx"
}"#);
        println!("  GET /solve     - Query params (comma-separated). Ejemplo:");
        println!("    /solve?ramos_pasados=CIT3313,CIT3211&ramos_prioritarios=CIT3413&horarios_preferidos=08:00-10:00&malla=MallaCurricular2020.xlsx&email=alumno%40ejemplo.cl");
        println!("  POST /rutacomoda/best - Body: {{ \"file_path\": \"/path/to/paths.json\" }} o incluir 'paths' array");
        println!("  GET /help      - Describe la API y devuelve ejemplos en JSON");
        println!("");
        println!("Nota: Para consultas complejas o datos privados use POST /solve con body JSON.");
    run_server(bind).await
}
