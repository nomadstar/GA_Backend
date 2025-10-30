// Módulo de alto nivel para la ejecución de la Ruta Crítica
// Declarar submódulos (archivos en la carpeta `src/algorithm`)
mod extract;
mod clique;
mod conflict;
mod pert;
mod ruta;

// Reexportar solo la API pública que quieres exponer desde aquí
pub use extract::extract_data;
// Note: la funcionalidad de lectura de malla queda centralizada en `crate::excel`.
// Exponemos aquí un wrapper compat (get_ramo_critico()) para no romper callers.

// Compat wrapper: invoca la versión de `excel` usando un nombre por defecto
// para no romper llamadas existentes que esperan `get_ramo_critico()` sin args.
pub fn get_ramo_critico() -> (std::collections::HashMap<String, crate::models::RamoDisponible>, String, bool) {
	// Nombre por defecto (legacy); `excel::resolve_datafile_paths` preferirá
	// archivos en `src/datafiles` cuando existan.
	crate::excel::get_ramo_critico("MiMalla.xlsx")
}

// Exponer las funciones de lectura de Excel que necesita el pipeline

// Reexportar funciones del planner (clique) y el orquestador (ruta)
pub use crate::algorithm::clique::{get_clique_with_user_prefs, get_clique_max_pond};
pub use crate::algorithm::ruta::ejecutar_ruta_critica_with_params;



// Nota: la API pública principal es `ruta::ejecutar_ruta_critica_with_params` y
// se reexporta arriba. Eliminamos la función wrapper para evitar lints
// en builds donde no se usa el helper genérico.

