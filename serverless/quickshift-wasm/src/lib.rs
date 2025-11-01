use worker::*;
use serde::{Deserialize, Serialize};
use base64;
use serde_json::json;
use std::error::Error;

// Declare submodules present under src/
mod algorithm;
mod api_json;
mod excel;
mod models;
mod storage;

// Entrada mínima esperada por el worker
#[derive(Deserialize)]
struct RunRequest {
    malla: Option<String>,
    ramos_prioritarios: Option<Vec<String>>,
    /// Opcional: contenido del XLSX codificado en base64. Si se proporciona, y la
    /// feature `excel` está habilitada, el worker intentará parsearlo desde memoria.
    malla_xlsx_b64: Option<String>,
}

#[derive(Serialize)]
struct RunResponse {
    status: String,
    message: String,
}

// Nota: aquí deberías importar tu crate `quickshift_core` que contenga la lógica
// algorítmica "wasm-friendly". Por ejemplo:
// use quickshift_core::compute_ruta_critica;

#[event(fetch)]
pub async fn main(mut req: Request, _env: Env, _ctx: worker::Context) -> Result<Response> {
    utils::set_panic_hook();

    // Sólo aceptar POST
    if !matches!(req.method(), Method::Post) {
        return Response::error("Method Not Allowed", 405);
    }

    // Intentar parsear JSON
    let body = req.text().await.unwrap_or_default();
    let parsed: Result<RunRequest, _> = serde_json::from_str(&body);

    match parsed {
        Ok(_run) => {
            // Si el request trae un xlsx en base64, intentar parsear (feature "excel")
            if let Some(b64) = _run.malla_xlsx_b64 {
                #[cfg(feature = "excel")]
                {
                    match base64::decode(&b64) {
                        Ok(bytes) => match excel::listar_hojas_malla_from_buffer(&bytes) {
                            Ok(sheets) => return Response::from_json(&json!({"status":"ok","sheets":sheets})),
                            Err(e) => return Response::error(&format!("Excel parse error: {}", e), 500),
                        },
                        Err(e) => return Response::error(&format!("base64 decode error: {}", e), 400),
                    }
                }

                #[cfg(not(feature = "excel"))]
                {
                    return Response::error("excel feature not enabled on this build", 400);
                }
            }

            // Aquí invocaríamos compute_ruta_critica(...) del crate core
            // Por ahora responderemos con un placeholder
            let resp = RunResponse {
                status: "ok".into(),
                message: "Worker listo — integra quickshift_core aquí".into(),
            };
            Response::from_json(&resp)
        }
        Err(e) => Response::error(&format!("Bad Request: {}", e), 400),
    }
}
