// Estructuras de datos principales

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct Seccion {
    pub codigo: String,
    pub nombre: String,
    pub seccion: String,
    pub horario: Vec<String>,
    pub profesor: String,
    pub codigo_box: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct RamoDisponible {
    /// ID único dentro de Malla2020 (1-57 típicamente)
    /// Usado para resolver dependencias en PERT
    pub id: i32,
    pub nombre: String,
    /// Código de la oferta (PA2025-1). Ej: "CIT2107"
    /// Usado para búsqueda en oferta y como referencia universal
    pub codigo: String,
    pub holgura: i32,
    pub numb_correlativo: i32,
    pub critico: bool,
    /// ID del ramo anterior (para dependencias PERT)
    /// Apunta al campo `id` del ramo precedente
    pub codigo_ref: Option<i32>,
    /// Porcentaje de aprobados (0.0 - 100.0). Se usará como estimador de dificultad inversa.
    /// Valores cercanos a 0 => muy difícil, cercanos a 100 => muy fácil.
    pub dificultad: Option<f64>,
    /// True si es un ramo electivo (puede elegirse entre opciones)
    pub electivo: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct PertNode {
    pub codigo: String,
    pub nombre: String,
    pub es: Option<i32>,  // Earliest Start
    pub ef: Option<i32>,  // Earliest Finish
    pub ls: Option<i32>,  // Latest Start
    pub lf: Option<i32>,  // Latest Finish
    pub h: Option<i32>,   // Holgura
}