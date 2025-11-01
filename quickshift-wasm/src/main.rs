// --- Sistema Generador de Horarios - Archivo principal ---

use quickshift::run_server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("=== Sistema Generador de Horarios (API) ===");
    let bind = "127.0.0.1:8080"; // cambia la dirección por lo que consideres pertinente.
    println!("Iniciando servidor en http://{}", bind);
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
    run_server(bind).await
}
