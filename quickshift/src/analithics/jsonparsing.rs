use serde_json::Value as JsonValue;
use std::error::Error;
use crate::api_json::InputParams;

/// ParsedFields represents the subset of fields we persist from a request.
pub struct ParsedFields {
    pub email: Option<String>,
    pub malla: Option<String>,
    pub student_ranking: Option<f64>,
    pub ramos_pasados: Option<String>,
    pub ramos_prioritarios: Option<String>,
    pub filtros_json: Option<String>,
}

/// Try to parse `request_json` as `InputParams` and extract a few fields.
/// Falls back to heuristic JSON extraction if parsing fails. Always returns
/// a `ParsedFields` with JSON-serialized vectors for the ramo lists.
pub fn extract_parsed_fields(request_json: &str) -> Result<ParsedFields, Box<dyn Error>> {
    let mut pf = ParsedFields { email: None, malla: None, student_ranking: None, ramos_pasados: None, ramos_prioritarios: None, filtros_json: None };

    if let Ok(parsed) = serde_json::from_str::<InputParams>(request_json) {
        pf.email = Some(parsed.email);
        pf.malla = Some(parsed.malla);
        pf.student_ranking = parsed.student_ranking;
        if !parsed.ramos_pasados.is_empty() { pf.ramos_pasados = Some(serde_json::to_string(&parsed.ramos_pasados)?); }
        if !parsed.ramos_prioritarios.is_empty() { pf.ramos_prioritarios = Some(serde_json::to_string(&parsed.ramos_prioritarios)?); }
        if let Some(f) = parsed.filtros { pf.filtros_json = Some(serde_json::to_string(&f)?); }
        return Ok(pf);
    }

    // fallback: heuristic extraction
    if let Ok(v) = serde_json::from_str::<JsonValue>(request_json) {
        if let Some(e) = v.get("email").and_then(|x| x.as_str()) { pf.email = Some(e.to_string()); }
        if let Some(m) = v.get("malla").and_then(|x| x.as_str()) { pf.malla = Some(m.to_string()); }
        if let Some(sr) = v.get("student_ranking").and_then(|x| x.as_f64()) { pf.student_ranking = Some(sr); }
        if let Some(rp) = v.get("ramos_pasados") { if let Ok(s) = serde_json::to_string(rp) { pf.ramos_pasados = Some(s); } }
        if let Some(rp) = v.get("ramos_prioritarios") { if let Ok(s) = serde_json::to_string(rp) { pf.ramos_prioritarios = Some(s); } }
        if let Some(f) = v.get("filtros") { if let Ok(s) = serde_json::to_string(f) { pf.filtros_json = Some(s); } }
    }
    Ok(pf)
}
