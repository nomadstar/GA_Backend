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
pub use crate::algorithm::clique::get_clique_with_user_prefs;
pub use crate::algorithm::ruta::ejecutar_ruta_critica_with_params;

// Helpers que exponen listas y resúmenes de ficheros de datos via el módulo
// `algorithm` (encapsulan acceso a `crate::excel` para que el server use la API
// del algoritmo en lugar de leer `src/datafiles` directamente).
use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::models::{RamoDisponible, Seccion};

/// Lista los archivos disponibles (mallas, ofertas, porcentajes) devolviendo
/// sólo los nombres de fichero.
pub fn list_datafiles() -> Result<(Vec<String>, Vec<String>, Vec<String>), Box<dyn Error>> {
	crate::excel::list_available_datafiles()
}

/// Resumen práctico de contenidos para una malla dada. Devuelve las rutas
/// resueltas y los objetos de alto nivel leídos (malla map, oferta vec, porcentajes map).
pub fn summarize_datafiles(malla_name: &str, sheet: Option<&str>) -> Result<(PathBuf, PathBuf, PathBuf, HashMap<String, RamoDisponible>, Vec<Seccion>, HashMap<String, (f64,f64)>), Box<dyn Error>> {
	let (malla_path, oferta_path, porcent_path) = crate::excel::resolve_datafile_paths(malla_name)?;

	// Leer primero la malla: si esto falla, no podemos continuar.
	let malla_path_str = malla_path.to_str().ok_or("malla path invalid UTF-8")?;
	let malla_map = match crate::excel::leer_malla_excel_with_sheet(malla_path_str, sheet) {
		Ok(m) => m,
		Err(e) => return Err(format!("failed to read malla '{}': {}", malla_path_str, e).into()),
	};

	// Intentar leer oferta; si falla degradamos a fallback vacío pero no abortamos.
	let oferta_path_str = oferta_path.to_str().ok_or("oferta path invalid UTF-8")?;
	let oferta = match crate::excel::leer_oferta_academica_excel(oferta_path_str) {
		Ok(o) => o,
		Err(e) => {
			eprintln!("WARN: no se pudo leer Oferta Académica '{}': {}. Usando fallback vacío.", oferta_path_str, e);
			Vec::new()
		}
	};

	// Intentar leer porcentajes; si falla devolvemos mapa vacío
	let porcent_path_str = porcent_path.to_str().ok_or("porcent path invalid UTF-8")?;
	let porcent = match crate::excel::leer_porcentajes_aprobados(porcent_path_str) {
		Ok(p) => p,
		Err(e) => {
			eprintln!("WARN: no se pudo leer Porcentajes '{}': {}. Usando fallback vacío.", porcent_path_str, e);
			HashMap::new()
		}
	};

	Ok((malla_path, oferta_path, porcent_path, malla_map, oferta, porcent))
}



// Nota: la API pública principal es `ruta::ejecutar_ruta_critica_with_params` y
// se reexporta arriba. Eliminamos la función wrapper para evitar lints
// en builds donde no se usa el helper genérico.

