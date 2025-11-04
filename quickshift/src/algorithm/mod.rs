// Módulo de alto nivel para la ejecución de la Ruta Crítica
// Declarar submódulos (archivos en la carpeta `src/algorithm`)
pub mod extract;
pub mod extract_optimizado;
pub mod extract_controller;
mod clique;
mod conflict;
mod pert;
mod ruta;

// Reexportar solo la API pública que quieres exponer desde aquí
pub use extract_controller::{extract_data};

// Reexportar funciones del planner (clique) y el orquestador (ruta)
pub use crate::algorithm::clique::get_clique_with_user_prefs;
pub use crate::algorithm::clique::get_clique_dependencies_only;
pub use crate::algorithm::ruta::ejecutar_ruta_critica_with_params;

// Compat wrapper: invoca la versión de `excel` usando un nombre por defecto
// para no romper llamadas existentes que esperan `get_ramo_critico()` sin args.
pub fn get_ramo_critico() -> (HashMap<String, RamoDisponible>, String, bool) {
	// Nombre por defecto (legacy); `excel::resolve_datafile_paths` preferirá
	// archivos en `src/datafiles` cuando existan.
	crate::excel::get_ramo_critico("MiMalla.xlsx")
}

// Helpers que exponen listas y resúmenes de ficheros de datos via el módulo
// `algorithm` (encapsulan acceso a `crate::excel` para que el server use la API
// del algoritmo en lugar de leer `src/datafiles` directamente).
use std::error::Error;
use std::path::PathBuf;
use std::collections::HashMap;
use crate::models::{RamoDisponible, Seccion};
use crate::excel::normalize_name;
use serde_json::json;

/// Une la malla, la oferta y los porcentajes intentando emparejar por nombre
/// normalizado. Devuelve una lista de objetos JSON ordenada por malla_codigo.
/// { malla_codigo, malla_nombre, oferta_codigo, oferta_codigo_box, oferta_nombre, pa_codigo, porcentaje, total, es_electivo }
pub fn merge_malla_oferta_porcentajes(
	malla_map: &HashMap<String, RamoDisponible>,
	oferta: &Vec<Seccion>,
	porcent: &HashMap<String, (f64,f64)>,
	porcent_names: &std::collections::HashMap<String, (String, f64, f64, bool)>,
) -> Vec<serde_json::Value> {
	// Construir índice de oferta por nombre normalizado -> Vec<Seccion>
	let mut oferta_index: std::collections::HashMap<String, Vec<&Seccion>> = std::collections::HashMap::new();
	for s in oferta.iter() {
		let key = normalize_name(&s.nombre);
		oferta_index.entry(key).or_default().push(s);
	}

	let mut out: Vec<serde_json::Value> = Vec::new();

	// Para cada ramo en la malla, buscar coincidencias en oferta por nombre
	for (mcode, ramo) in malla_map.iter() {
		let rname_norm = normalize_name(&ramo.nombre);
		if let Some(matches) = oferta_index.get(&rname_norm) {
			for s in matches.iter() {
				// Buscar porcentaje por nombre normalizado (más confiable que por código_box)
				if let Some((pa_code, pct, tot, _is_electivo)) = porcent_names.get(&rname_norm) {
					// Match encontrado en porcent_names por nombre
					out.push(json!({
						"malla_codigo": mcode,
						"malla_nombre": ramo.nombre,
						"oferta_codigo": s.codigo,
						"oferta_codigo_box": s.codigo_box,
						"oferta_nombre": s.nombre,
						"pa_codigo": pa_code,
						"porcentaje": *pct,
						"total": *tot
					}));
				} else if let Some((pct, tot)) = porcent.get(&s.codigo_box) {
					// Fallback: intentar por codigo_box si existe en porcent
					out.push(json!({
						"malla_codigo": mcode,
						"malla_nombre": ramo.nombre,
						"oferta_codigo": s.codigo,
						"oferta_codigo_box": s.codigo_box,
						"oferta_nombre": s.nombre,
						"pa_codigo": s.codigo_box.clone(),
						"porcentaje": *pct,
						"total": *tot
					}));
				} else {
					// No se encontró porcentaje
					out.push(json!({
						"malla_codigo": mcode,
						"malla_nombre": ramo.nombre,
						"oferta_codigo": s.codigo,
						"oferta_codigo_box": s.codigo_box,
						"oferta_nombre": s.nombre,
						"pa_codigo": serde_json::Value::Null,
						"porcentaje": serde_json::Value::Null,
						"total": serde_json::Value::Null
					}));
				}
			}
		} else {
			// Intentar emparejar directamente PA -> malla por nombre como fallback
			if let Some((pa_code, pct, tot, es_electivo)) = porcent_names.get(&rname_norm) {
				out.push(json!({
					"malla_codigo": mcode,
					"malla_nombre": ramo.nombre,
					"oferta_codigo": serde_json::Value::Null,
					"oferta_codigo_box": serde_json::Value::Null,
					"oferta_nombre": serde_json::Value::Null,
					"pa_codigo": pa_code,
					"porcentaje": *pct,
					"total": *tot,
					"es_electivo": es_electivo
				}));
			} else {
				// No encontrado en oferta ni en PA por nombre: fila vacía
				out.push(json!({
					"malla_codigo": mcode,
					"malla_nombre": ramo.nombre,
					"oferta_codigo": serde_json::Value::Null,
					"oferta_codigo_box": serde_json::Value::Null,
					"oferta_nombre": serde_json::Value::Null,
					"pa_codigo": serde_json::Value::Null,
					"porcentaje": serde_json::Value::Null,
					"total": serde_json::Value::Null
				}));
			}
		}
	}

	// **ORDENAR POR MALLA_CODIGO NUMÉRICO**
	out.sort_by(|a, b| {
		let a_code_str = a.get("malla_codigo")
			.and_then(|v| v.as_str())
			.unwrap_or("ZZZZZ");
		let b_code_str = b.get("malla_codigo")
			.and_then(|v| v.as_str())
			.unwrap_or("ZZZZZ");

		let a_num = a_code_str.parse::<i32>().ok();
		let b_num = b_code_str.parse::<i32>().ok();

		match (a_num, b_num) {
			(Some(an), Some(bn)) => an.cmp(&bn),
			(Some(_), None) => std::cmp::Ordering::Less,
			(None, Some(_)) => std::cmp::Ordering::Greater,
			(None, None) => a_code_str.cmp(b_code_str),
		}
	});

	out
}

/// Lista los archivos disponibles (mallas, ofertas, porcentajes) devolviendo
/// sólo los nombres de fichero.
pub fn list_datafiles() -> Result<(Vec<String>, Vec<String>, Vec<String>), Box<dyn Error>> {
	crate::excel::list_available_datafiles()
}

/// Resumen práctico de contenidos para una malla dada. Devuelve las rutas
/// resueltas y los objetos de alto nivel leídos (malla map, oferta vec, porcentajes map).
pub fn summarize_datafiles(malla_name: &str, sheet: Option<&str>) -> Result<(PathBuf, PathBuf, PathBuf, HashMap<String, RamoDisponible>, Vec<Seccion>, HashMap<String, (f64,f64)>, std::collections::HashMap<String, (String, f64, f64, bool)>), Box<dyn Error>> {
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

	// Intentar leer porcentajes; si falla devolvemos mapa vacío. Usamos
	// la variante que también intenta extraer nombres para matching por nombre.
	let porcent_path_str = porcent_path.to_str().ok_or("porcent path invalid UTF-8")?;
	let (porcent, mut porcent_names) = match crate::excel::leer_porcentajes_aprobados_con_nombres(porcent_path_str) {
		Ok((p, pn)) => (p, pn),
		Err(e) => {
			eprintln!("WARN: no se pudo leer Porcentajes '{}': {}. Usando fallback vacío.", porcent_path_str, e);
			(HashMap::new(), std::collections::HashMap::new())
		}
	};

	// Si porcent_names está vacío (porque PA no tiene columna "nombre"),
	// enriquecerlo usando nombres de Malla
	if porcent_names.is_empty() && !porcent.is_empty() {
		crate::excel::enrich_porcent_names_from_malla(&mut porcent_names, &porcent, &malla_map);
	}

	Ok((malla_path, oferta_path, porcent_path, malla_map, oferta, porcent, porcent_names))
}



// Nota: la API pública principal es `ruta::ejecutar_ruta_critica_with_params` y
// se reexporta arriba. Eliminamos la función wrapper para evitar lints
// en builds donde no se usa el helper genérico.