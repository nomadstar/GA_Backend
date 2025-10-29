// Módulo de alto nivel para la ejecución de la Ruta Crítica
// Declarar submódulos (archivos en la carpeta `src/algorithm`)
mod extract;
mod clique;
mod conflict;
mod pert;
mod ruta;

// Reexportar solo la API pública que quieres exponer desde aquí
pub use extract::{get_ramo_critico, extract_data};

// Exponer las funciones de lectura de Excel que necesita el pipeline
pub use crate::excel::{leer_malla_excel, leer_porcentajes_aprobados, leer_oferta_academica_excel};

// Reexportar funciones del planner (clique) y el orquestador (ruta)
pub use crate::algorithm::clique::{get_clique_with_user_prefs, get_clique_max_pond};
pub use crate::algorithm::ruta::ejecutar_ruta_critica_with_params;



// Nota: la API pública principal es `ruta::ejecutar_ruta_critica_with_params` y
// se reexporta arriba. Eliminamos la función wrapper para evitar lints
// en builds donde no se usa el helper genérico.

