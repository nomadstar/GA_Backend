// --- Sistema Generador de Horarios - Archivo principal ---

use quickshift::run_server;
use std::env;
use quickshift::algorithm::extract_controller; // <-- agregado

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("=== Sistema Generador de Horarios (API) ===");

    // Bind a 0.0.0.0 y puerto desde env PORT (Railway la expone)
    let port: u16 = env::var("PORT").unwrap_or_else(|_| "8080".into()).parse().unwrap_or(8080);
    let bind = format!("0.0.0.0:{}", port);

    println!("Iniciando servidor en http://{}", bind);
    // Leer variable de entorno USE_OPTIMIZED (true/false). Por defecto true.
    let use_opt = env::var("USE_OPTIMIZED").unwrap_or_else(|_| "true".into());
    let use_opt_bool = matches!(use_opt.to_lowercase().as_str(), "1" | "true" | "yes" | "y");
    extract_controller::set_use_optimized(use_opt_bool);
    println!("");
    println!("Endpoints disponibles:");
    println!("  POST /solve    - Body JSON. Ejemplo (use 'malla' y opcional 'sheet' para seleccionar hoja interna):");
    println!("{}", r#"{
    "email": "alumno@ejemplo.cl",
    "ramos_pasados": ["CIT3313", "CIT3211"],
    "ramos_prioritarios": ["CIT3313"],
    "horarios_preferidos": ["08:00-10:00"],
    "malla": "MallaCurricular2020.xlsx",
    "sheet": "Malla 2020"
}"#);
    println!("  GET /solve     - Query params (comma-separated). Ejemplo:");
    println!("    /solve?ramos_pasados=CIT3313,CIT3211&ramos_prioritarios=CIT3413&horarios_preferidos=08:00-10:00&malla=MallaCurricular2020.xlsx&sheet=Malla%202020&email=alumno%40ejemplo.cl");
    println!("{}", r#"  POST /rutacomoda/best - Body: { "file_path": "/path/to/paths.json" } o incluir 'paths' array"#);
    println!("  POST /rutacritica/run - Ejecuta el orquestador con body JSON (igual que POST /solve)");
    println!("  GET /datafiles - Lista archivos disponibles en src/datafiles");
    println!("  GET /datafiles/content?malla=MiMalla.xlsx[&sheet=Hoja]");
    println!("      - Devuelve resumen de malla/oferta/porcentajes y lista de hojas internas de la malla");
    println!("  POST /students  - Guarda un perfil de estudiante (body JSON, se indexa por email)");
    println!("  GET /help       - Describe la API y muestra ejemplos en JSON");
    println!("");
    println!("Nota: GET /solve es una versión ligera (parametros por query). Para datos privados o estructuras complejas use POST /solve o POST /rutacritica/run con body JSON.");
    run_server(&bind).await
}
