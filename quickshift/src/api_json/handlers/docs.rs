use actix_web::{HttpResponse, Responder};

const OPENAPI_JSON: &str = include_str!("../../openapi.json");
const SWAGGER_HTML: &str = include_str!("../../swagger.html");

pub async fn openapi_json_handler() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json; charset=utf-8")
        .body(OPENAPI_JSON)
}

pub async fn swagger_ui_handler() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(SWAGGER_HTML)
}

pub async fn root_redirect_handler() -> impl Responder {
    HttpResponse::Found()
        .append_header((actix_web::http::header::LOCATION, "/api-docs"))
        .finish()
}
