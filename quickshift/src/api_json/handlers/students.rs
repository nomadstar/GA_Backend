use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use std::fs::OpenOptions;
use std::path::Path;
use std::fs::create_dir_all;
use std::io::Write;
use crate::api_json::InputParams;

pub async fn save_student_handler(body: web::Json<serde_json::Value>) -> impl Responder {
    let body_value = body.into_inner();
    let json_str = match serde_json::to_string(&body_value) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("invalid JSON body: {}", e)})),
    };

    let student = match crate::api_json::parse_and_resolve_ramos(&json_str, Some(".")) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({"error": format!("failed to parse input: {}", e)})),
    };

    if student.email.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({"error": "email is required"}));
    }

    let data_dir = "data";
    if let Err(e) = create_dir_all(data_dir) {
        return HttpResponse::InternalServerError().json(json!({"error": format!("failed to create data dir: {}", e)}));
    }

    let file_path = format!("{}/students.json", data_dir);
    let mut students: Vec<InputParams> = Vec::new();
    if Path::new(&file_path).exists() {
        match std::fs::read_to_string(&file_path) {
            Ok(contents) if !contents.trim().is_empty() => {
                match serde_json::from_str::<Vec<InputParams>>(&contents) {
                    Ok(v) => students = v,
                    Err(_) => { /* ignore and start fresh */ }
                }
            }
            _ => { /* empty file or read error -> start fresh */ }
        }
    }

    students.retain(|s| s.email.to_lowercase() != student.email.to_lowercase());
    students.push(student);

    match OpenOptions::new().write(true).create(true).truncate(true).open(&file_path) {
        Ok(mut f) => {
            match serde_json::to_string_pretty(&students) {
                Ok(text) => {
                    if let Err(e) = f.write_all(text.as_bytes()) { return HttpResponse::InternalServerError().json(json!({"error": format!("failed to write students: {}", e)})); }
                }
                Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to serialize students: {}", e)})),
            }
        }
        Err(e) => return HttpResponse::InternalServerError().json(json!({"error": format!("failed to open file: {}", e)})),
    }

    HttpResponse::Ok().json(json!({"status": "ok", "count": students.len()}))
}
