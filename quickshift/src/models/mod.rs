// Estructuras de datos principales

/// Filtros opcionales del usuario (Reglas 3-6 en Plan.md)
/// Todos los campos son opcionales; si no se especifican, se ignoran los filtros
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Default)]
pub struct UserFilters {
    /// Filtro 3: Días/horarios libres
    pub dias_horarios_libres: Option<DiaHorariosLibres>,
    /// Filtro 4: Ventana entre actividades
    pub ventana_entre_actividades: Option<VentanaEntreActividades>,
    /// Filtro 5: Preferencias de Profesores
    pub preferencias_profesores: Option<PreferenciasProfesores>,
    /// Filtro 6: Balance entre líneas de formación
    pub balance_lineas: Option<BalanceLineas>,

}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct DiaHorariosLibres {
    pub habilitado: bool,
    pub dias_libres_preferidos: Option<Vec<String>>, // ["LU", "MA", ..., "VI"]
    pub minimizar_ventanas: Option<bool>,
    pub ventana_ideal_minutos: Option<i32>,
    /// Franjas explícitas prohibidas, por ejemplo: ["LU 08:30-10:00", "VI 14:00-18:00"].
    /// Si se especifican, se tratan como bloques prohibidos y se excluirán secciones que solapen.
    pub franjas_prohibidas: Option<Vec<String>>,
    /// Si true, evitar secciones marcadas como "Sin horario".
    pub no_sin_horario: Option<bool>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct VentanaEntreActividades {
    pub habilitado: bool,
    pub minutos_entre_clases: Option<i32>, // default: 15
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PreferenciasProfesores {
    pub habilitado: bool,
    pub profesores_preferidos: Option<Vec<String>>,
    pub profesores_evitar: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BalanceLineas {
    pub habilitado: bool,
    pub lineas: Option<std::collections::HashMap<String, f64>>, // {"informatica": 0.6, "telecomunicaciones": 0.4}
}

// Note: carga (max ramos) is enforced as a fixed cap of 6 per semester in the algorithm.

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
    /// IDs de los ramos prerequisitos (para dependencias PERT)
    /// Lista de IDs de ramos que deben ser aprobados antes de tomar este
    pub requisitos_ids: Vec<i32>,
    /// Porcentaje de aprobados (0.0 - 100.0). Se usará como estimador de dificultad inversa.
    /// Valores cercanos a 0 => muy difícil, cercanos a 100 => muy fácil.
    pub dificultad: Option<f64>,
    /// True si es un ramo electivo (puede elegirse entre opciones)
    pub electivo: bool,
    /// Semestre curricular (1 = S1, 2 = S2, etc.)
    pub semestre: Option<i32>,
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