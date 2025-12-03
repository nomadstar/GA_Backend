use actix_multipart::Multipart;
use futures_util::stream::StreamExt;
use serde_json::json;
use tokio::io::AsyncWriteExt;
use crate::algorithm::{list_datafiles, summarize_datafiles};
use actix_web::{web, HttpResponse, Responder};

pub async fn datafiles_list_handler() -> impl Responder {
    match list_datafiles() {
        Ok((mallas, ofertas, porcentajes)) => HttpResponse::Ok().json(json!({"mallas": mallas, "ofertas": ofertas, "porcentajes": porcentajes})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to list datafiles: {}", e)})),
    }
}

pub async fn datafiles_upload_handler(mut payload: Multipart) -> impl Responder {
    let base = std::path::Path::new("src/datafiles");
    if let Err(e) = std::fs::create_dir_all(base) {
        return HttpResponse::InternalServerError().json(json!({"error": format!("failed to create datafiles dir: {}", e)}));
    }

    let mut saved: Vec<String> = Vec::new();
    while let Some(field_res) = payload.next().await {
        match field_res {
            Ok(mut field) => {
                // Try to read filename from content-disposition
                let filename = field.content_disposition()
                    .get_filename()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("upload-{}.dat", chrono::Utc::now().timestamp_millis()));

                // Sanitize filename a bit
                if filename.contains("..") {
                    continue;
                }

                let filepath = base.join(&filename);
                match tokio::fs::File::create(&filepath).await {
                    Ok(mut f) => {
                        while let Some(chunk) = field.next().await {
                            match chunk {
                                Ok(bytes) => {
                                    if let Err(e) = f.write_all(&bytes).await {
                                        eprintln!("failed to write upload chunk: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    eprintln!("upload stream error: {}", e);
                                    break;
                                }
                            }
                        }
                        saved.push(filename);
                    }
                    Err(e) => {
                        eprintln!("failed to create upload file: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("multipart field error: {}", e);
            }
        }
    }

    HttpResponse::Ok().json(json!({"status": "ok", "saved": saved}))
}

pub async fn datafiles_download_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let name = match query.get("name") {
        Some(n) if !n.trim().is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().json(json!({"error": "missing name parameter"})),
    };

    if name.contains("..") { return HttpResponse::BadRequest().json(json!({"error": "invalid name"})); }
    let path = std::path::Path::new("src/datafiles").join(&name);
    if !path.exists() { return HttpResponse::NotFound().json(json!({"error": "file not found"})); }

    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            // try to set mime by extension (simple mapping)
            let mime = match path.extension().and_then(std::ffi::OsStr::to_str) {
                Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                Some("xls") => "application/vnd.ms-excel",
                _ => "application/octet-stream",
            };
            HttpResponse::Ok()
                .content_type(mime)
                .append_header((actix_web::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", name)))
                .body(bytes)
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to read file: {}", e)})),
    }
}

pub async fn datafiles_delete_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let name = match query.get("name") {
        Some(n) if !n.trim().is_empty() => n.clone(),
        _ => return HttpResponse::BadRequest().json(json!({"error": "missing name parameter"})),
    };
    if name.contains("..") { return HttpResponse::BadRequest().json(json!({"error": "invalid name"})); }
    let path = std::path::Path::new("src/datafiles").join(&name);
    if !path.exists() { return HttpResponse::NotFound().json(json!({"error": "file not found"})); }
    match tokio::fs::remove_file(&path).await {
        Ok(_) => HttpResponse::Ok().json(json!({"status": "deleted", "name": name})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to delete file: {}", e)})),
    }
}

pub async fn datafiles_content_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let qm = query.into_inner();
    let raw_malla = match qm.get("malla").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) }) {
        Some(s) => s,
        None => return HttpResponse::BadRequest().json(json!({"error": "malla parameter required"})),
    };

    let mut malla = raw_malla.clone();
    let mut sheet_from_brackets: Option<String> = None;
    if let Some(start) = raw_malla.find('[') {
        if let Some(end) = raw_malla.rfind(']') {
            let inside = &raw_malla[start+1..end];
            if !inside.trim().is_empty() {
                sheet_from_brackets = Some(inside.to_string());
                malla = raw_malla[..start].to_string();
            }
        }
    }

    let sheet_qparam: Option<String> = qm.get("sheet").and_then(|s| if s.trim().is_empty() { None } else { Some(s.clone()) });
    let sheet_opt: Option<String> = match (sheet_qparam, sheet_from_brackets) {
        (Some(s), _) => Some(s),
        (None, Some(s)) => Some(s),
        _ => None,
    };

    if let Ok((available_mallas, _ofertas, _porc)) = list_datafiles() {
        if !available_mallas.iter().any(|x| x == &malla) {
            return HttpResponse::BadRequest().json(json!({"error": "malla not found among available datafiles", "available": available_mallas}));
        }
    }

    match summarize_datafiles(&malla, sheet_opt.as_deref()) {
        Ok(v) => HttpResponse::Ok().json(v),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("failed to summarize datafiles: {}", e)})),
    }
}

pub async fn oferta_summary_handler(query: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let oferta_file = match query.get("oferta") {
        Some(o) if !o.trim().is_empty() => o.clone(),
        _ => "OA2024.xlsx".to_string(),
    };

    eprintln!("📋 Generando resumen de oferta: {}", oferta_file);

    match crate::excel::oferta::resumen_oferta_academica(&oferta_file) {
        Ok(resumen) => {
            let total_secciones: usize = resumen.iter().map(|(_, count)| count).sum();
            let response = json!({
                "archivo": oferta_file,
                "total_ramos": resumen.len(),
                "total_secciones": total_secciones,
                "ramos": resumen.iter().map(|(nombre, count)| json!({
                    "nombre": nombre,
                    "secciones": count
                })).collect::<Vec<_>>()
            });
            HttpResponse::Ok().json(response)
        }
        Err(e) => {
            eprintln!("❌ Error al generar resumen: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "error": format!("failed to generate oferta summary: {}", e)
            }))
        }
    }
}
