use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::excel::asignatura_from_nombre;

#[derive(Debug, Serialize, Deserialize)]
pub struct InputParams {
	pub email: String,
	pub ramos_pasados: Vec<String>,
	pub ramos_prioritarios: Vec<String>,
	pub horarios_preferidos: Vec<String>,
	// Optional: which curricular map to use. Example values: "MallaCurricular2010.xlsx", "MallaCurricular2018.xlsx", "MallaCurricular2020.xlsx"
	pub malla: Option<String>,
}

pub fn parse_json_input(json_str: &str) -> Result<InputParams, serde_json::Error> {
	serde_json::from_str::<InputParams>(json_str)
}

/// Parsea el JSON de entrada y, si se especifica `malla`, intentará resolver
/// ramos que no parezcan códigos (p. ej. nombres completos) usando la función
/// `asignatura_from_nombre` que busca en la hoja de oferta/malla la fila cuyo
/// "Nombre Asignado" coincide y devuelve la columna "Asignatura".
///
/// Parámetros:
/// - `json_str`: JSON de entrada igual que para `parse_json_input`.
/// - `base_dir`: directorio base opcional donde buscar el fichero `malla` si es
///   un nombre relativo.
pub fn parse_and_resolve_ramos<P: AsRef<Path>>(json_str: &str, base_dir: Option<P>) -> Result<InputParams, Box<dyn std::error::Error>> {
	// devolvemos a una versión parametrizable para facilitar pruebas (inyección de resolver)
	parse_and_resolve_ramos_with_resolver(json_str, base_dir, |p, name| asignatura_from_nombre(p, name))
}

/// Versión parametrizable para pruebas: recibe un `resolver` que intenta mapear
/// un `nombre_asignado` a la `Asignatura` (código). Esto permite mockear sin
/// depender de un archivo Excel real en los tests.
pub fn parse_and_resolve_ramos_with_resolver<P, F>(json_str: &str, base_dir: Option<P>, resolver: F) -> Result<InputParams, Box<dyn std::error::Error>>
where
	P: AsRef<Path>,
	F: Fn(&Path, &str) -> Result<Option<String>, Box<dyn std::error::Error>>,
{
	let mut params = parse_json_input(json_str)?;

	// Si no se especificó malla, devolvemos lo parseado sin resolución adicional.
	let malla_name = match &params.malla {
		Some(m) if !m.trim().is_empty() => m.clone(),
		_ => return Ok(params),
	};

	let malla_path: PathBuf = match base_dir {
		Some(b) => b.as_ref().join(malla_name.clone()),
		None => PathBuf::from(malla_name.clone()),
	};

	// heurística simple: si la cadena contiene un dígito la consideramos código
	fn looks_like_code(s: &str) -> bool {
		s.chars().any(|c| c.is_ascii_digit())
	}

	let resolve_one = |r: String| -> String {
		if looks_like_code(&r) {
			return r;
		}
		match resolver(&malla_path, &r) {
			Ok(Some(asig)) => asig,
			Ok(None) => r,
			Err(_) => r,
		}
	};

	params.ramos_pasados = params.ramos_pasados.into_iter().map(resolve_one).collect();
	params.ramos_prioritarios = params.ramos_prioritarios.into_iter().map(resolve_one).collect();

	Ok(params)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_and_resolve_ramos_with_mock() {
		// JSON con nombres (no códigos) en ramos_pasados y ramos_prioritarios
		let json_data = r#"
		{
			"email": "juan.perez@example.com",
			"ramos_pasados": ["Algebra y Geometría", "Calculo 1", "Programación"],
			"ramos_prioritarios": ["Programación Avanzada", "Calculo 2"],
			"horarios_preferidos": ["08:00-10:00"],
			"malla": "MallaCurricularTest.xlsx"
		}
		"#;

		// mock resolver: mapea algunos nombres a códigos conocidos
		let resolver = |_p: &Path, name: &str| -> Result<Option<String>, Box<dyn std::error::Error>> {
			let lower = name.to_lowercase();
			if lower.contains("programación avanzada") { return Ok(Some("CIT9999".to_string().into())); }
			if lower.contains("programación") { return Ok(Some("CIT1001".to_string().into())); }
			if lower.contains("algebra") { return Ok(Some("MAT1000".to_string().into())); }
			// Calculo 1/2 no los resolvemos en el mock
			Ok(None)
		};

		let params = parse_and_resolve_ramos_with_resolver(json_data, Some("."), resolver).unwrap();

		// Comprobaciones: los ramos que el mock pudo resolver deben aparecer convertidos
		assert!(params.ramos_pasados.contains(&"MAT1000".to_string()));
		assert!(params.ramos_pasados.contains(&"CIT1001".to_string()));
		assert!(params.ramos_prioritarios.contains(&"CIT9999".to_string()));
		// Los que no se resolvieron deben quedar como estaban (Calculo 1 se mantiene)
		assert!(params.ramos_pasados.contains(&"Calculo 1".to_string()));
	}

	#[test]
	fn test_parse_json_input() {
		let json_data = r#"
		{
			"email": "alumno@ejemplo.cl",
			"ramos_pasados": ["CIT3313", "CIT3211"],
			"ramos_prioritarios": ["CIT3313", "CIT3413"],
			"horarios_preferidos": ["08:00-10:00", "14:00-16:00"],
			"malla": "MallaCurricular2020.xlsx"
		}
		"#;

		let params = parse_json_input(json_data).unwrap();
		assert_eq!(params.ramos_pasados, vec!["CIT3313", "CIT3211"]);
		assert_eq!(params.ramos_prioritarios, vec!["CIT3313", "CIT3413"]);
	assert_eq!(params.horarios_preferidos, vec!["08:00-10:00", "14:00-16:00"]);
	assert_eq!(params.malla.unwrap(), "MallaCurricular2020.xlsx");
	}
}
