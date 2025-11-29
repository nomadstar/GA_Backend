use actix_web::{web, App, HttpResponse, HttpServer, Responder, HttpRequest};
use actix_cors::Cors;
use actix_multipart::Multipart;
use serde_json::json;
use crate::algorithm::{extract_data, get_clique_with_user_prefs};
use crate::models::Seccion;
use crate::api_json::InputParams;
use std::sync::OnceLock;
use std::sync::Arc;
use tokio::sync::Semaphore;
use num_cpus;
use crate::server_handlers;
// Note: periodic persistence was removed for now to avoid runtime complexity.

// Lightweight wrappers delegate heavy logic to `server_handlers` modules.

async fn solve_handler(req: HttpRequest, body: web::Json<serde_json::Value>) -> impl Responder {
    crate::server_handlers::solve::solve_handler(req, body).await
}

/// Handler para obtener los mejores caminos desde un JSON de `PathsOutput` o un
/// `file_path` que apunte a un JSON en disco generado por Ruta crítica.
async fn rutacomoda_best_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    crate::server_handlers::rutacritica::rutacomoda_best_handler(body).await
}

async fn rutacritica_run_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    crate::server_handlers::rutacritica::rutacritica_run_handler(body).await
}

/// POST /rutacritica/run-dependencies-only
/// Ejecuta la ruta crítica considerando SOLO dependencias, sin verificar conflictos de horarios.
/// Útil para validar el orden correcto de cursos sin restricciones de compatibilidad de horarios.
async fn rutacritica_run_dependencies_only_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    crate::server_handlers::rutacritica::rutacritica_run_dependencies_only_handler(body).await
}

// Analytics HTTP handlers
async fn anal_ramos_pasados_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::analytics::anal_ramos_pasados_handler(query).await
}

async fn anal_ranking_handler() -> impl Responder {
    crate::api_json::handlers::analytics::anal_ranking_handler().await
}

async fn anal_count_users_handler() -> impl Responder {
    crate::api_json::handlers::analytics::anal_count_users_handler().await
}

async fn anal_filtros_handler() -> impl Responder {
    crate::api_json::handlers::analytics::anal_filtros_handler().await
}

async fn anal_ramos_recomendados_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::analytics::anal_ramos_recomendados_handler(query).await
}

/// POST /students
/// Guarda los datos del estudiante en `data/students.json`. Si ya existe un
/// estudiante con el mismo correo, lo sustituye.
async fn save_student_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    crate::api_json::handlers::students::save_student_handler(body).await
}

// OpenAPI and Swagger UI are served from the `api_json::handlers::docs` module.

// Nuevo handler para servir el OpenAPI JSON
async fn openapi_json_handler() -> impl Responder {
    crate::api_json::handlers::openapi_json_handler().await
}

// Nuevo handler para servir la página Swagger UI (carga JSON desde /api-doc/openapi.json)
async fn swagger_ui_handler() -> impl Responder {
    crate::api_json::handlers::swagger_ui_handler().await
}

// Redirige `/` a la UI de documentación (`/api-docs`)
async fn root_redirect_handler() -> impl Responder {
    crate::api_json::handlers::root_redirect_handler().await
}

pub async fn run_server(bind_addr: &str) -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            // CORS: During development allow localhost origins so browser clients
            // (served from different ports) can call the API. In production tighten this.
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
                    .allowed_headers(vec![
                        actix_web::http::header::AUTHORIZATION,
                        actix_web::http::header::ACCEPT,
                        actix_web::http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600)
            )
            // Initialize analytics DB (best-effort)
            .app_data({
                // call init_db here in closure side-effect: we call it once when app is built
                if let Err(e) = crate::analithics::init_db() {
                    eprintln!("analytics init failed: {}", e);
                }
                // analytics initialization only (no background persistence started here)
                web::Data::new(())
            })
            .route("/", web::get().to(root_redirect_handler))
            .route("/solve", web::post().to(solve_handler))
            .route("/solve", web::get().to(solve_get_handler))
                .route("/students", web::post().to(save_student_handler))
            // Analytics routes
            .route("/analithics/ramos_pasados", web::get().to(anal_ramos_pasados_handler))
            .route("/analithics/ranking_por_estudiante", web::get().to(anal_ranking_handler))
            .route("/analithics/count_users", web::get().to(anal_count_users_handler))
            .route("/analithics/filtros_mas_solicitados", web::get().to(anal_filtros_handler))
            .route("/analithics/ramos_mas_recomendados", web::get().to(anal_ramos_recomendados_handler))
            // Cache stats endpoints (latest and recent)
            .route("/analithics/cache_stats/latest", web::get().to(crate::server_handlers::analithics::cache_stats_latest))
            .route("/analithics/cache_stats/recent", web::get().to(crate::server_handlers::analithics::cache_stats_recent))
            .route("/rutacomoda/best", web::post().to(rutacomoda_best_handler))
            .route("/rutacritica/run", web::post().to(rutacritica_run_handler))
            .route("/rutacritica/run-dependencies-only", web::post().to(rutacritica_run_dependencies_only_handler))
            .route("/datafiles", web::get().to(datafiles_list_handler))
            .route("/datafiles", web::delete().to(datafiles_delete_handler))
            .route("/datafiles/upload", web::post().to(datafiles_upload_handler))
            .route("/datafiles/download", web::get().to(datafiles_download_handler))
            .route("/datafiles/content", web::get().to(datafiles_content_handler))
            .route("/datafiles/debug/pa-names", web::get().to(debug_pa_names_handler))
            .route("/help", web::get().to(help_handler))
            // Registrar rutas de documentación SWAGGER
            .route("/api-doc/openapi.json", web::get().to(openapi_json_handler))
            .route("/api-docs", web::get().to(swagger_ui_handler))
    })
    .bind(bind_addr)?
    .run()
    .await
}

/// GET /datafiles
/// Lista los nombres de archivos MC, OA y PA disponibles en `src/datafiles`.
async fn datafiles_list_handler() -> impl Responder {
    crate::api_json::handlers::datafiles::datafiles_list_handler().await
}

/// POST /datafiles/upload
/// multipart/form-data upload; field(s) with files will be written to `src/datafiles/<filename>`
async fn datafiles_upload_handler(mut payload: Multipart) -> impl Responder {
    crate::api_json::handlers::datafiles::datafiles_upload_handler(payload).await
}

/// GET /datafiles/download?name=archivo.xlsx
async fn datafiles_download_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::datafiles::datafiles_download_handler(query).await
}

/// DELETE /datafiles?name=archivo.xlsx
async fn datafiles_delete_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::datafiles::datafiles_delete_handler(query).await
}

/// GET /datafiles/content?malla=MiMalla.xlsx
/// Devuelve un resumen de los contenidos (primeros elementos) de MALLA, OA y PA
async fn datafiles_content_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::datafiles::datafiles_content_handler(query).await
}

/// GET /solve handler: acepta parámetros simples en query string.
/// Parámetros esperados (comma-separated lists):
/// - ramos_pasados
/// - ramos_prioritarios
/// - horarios_preferidos
/// - malla
/// - email
async fn solve_get_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::server_handlers::solve::solve_get_handler(query).await
}

async fn help_handler() -> impl Responder {
    crate::server_handlers::docs::help_handler().await
}

/// DEBUG: GET /datafiles/debug/pa-names
/// Muestra un sample del índice de nombres normalizados extraídos del PA para diagnóstico
async fn debug_pa_names_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    crate::api_json::handlers::debug::debug_pa_names_handler(query).await
}
