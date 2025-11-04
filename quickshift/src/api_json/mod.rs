use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::excel::asignatura_from_nombre;
use crate::models::UserFilters;

/// Parámetros de entrada para la ejecución de Ruta Crítica
///
/// # Estructura del JSON esperado:
/// ```json
/// {
///   "email": "estudiante@example.com",
///   "ramos_pasados": ["CBM1000", "CBM1001"],
///   "ramos_prioritarios": ["CIT3313"],
///   "horarios_preferidos": ["08:00-10:00"],
///   "malla": "MiMalla.xlsx",
///   "sheet": null,
///   "student_ranking": 0.75,
///   "ranking": null,
///   "filtros": {
///     "dias_horarios_libres": {
///       "habilitado": false,
///       "dias_libres_preferidos": ["VI"],
///       "minimizar_ventanas": true,
///       "ventana_ideal_minutos": 30
///     },
///     "ventana_entre_actividades": {
///       "habilitado": false,
///       "minutos_entre_clases": 15
///     },
///     "preferencias_profesores": {
///       "habilitado": false,
///       "profesores_preferidos": ["Dr. García"],
///       "profesores_evitar": []
///     },
///     "balance_lineas": {
///       "habilitado": false,
///       "lineas": {
///         "informatica": 0.6,
///         "telecomunicaciones": 0.4
///       }
///     }
///   }
/// }
/// ```
///
/// # Campos:
/// - `email`: Email del estudiante (requerido)
/// - `ramos_pasados`: Lista de códigos/nombres de ramos ya aprobados (Regla 0: Prerequisitos)
/// - `ramos_prioritarios`: Ramos que el estudiante quiere priorizar
/// - `horarios_preferidos`: Rangos horarios preferidos (formato "HH:MM-HH:MM")
/// - `malla`: Nombre del archivo de Malla Curricular (requerido)
/// - `sheet`: Hoja interna dentro del workbook (opcional)
/// - `student_ranking`: Ranking académico como percentil 0.0-1.0 (Regla 2: Probabilidad aprobación)
/// - `ranking`: Preferencias de ranking del usuario
/// - `filtros`: Filtros opcionales del usuario (Reglas 3-6). Cada filtro tiene `habilitado: true/false`
#[derive(Debug, Serialize, Deserialize)]
pub struct InputParams {
	pub email: String,
	pub ramos_pasados: Vec<String>,
	pub ramos_prioritarios: Vec<String>,
	pub horarios_preferidos: Vec<String>,
	// Required: which curricular map to use. Example values: "MallaCurricular2010.xlsx", "MallaCurricular2018.xlsx", "MallaCurricular2020.xlsx"
	pub malla: String,
	// Optional: which internal sheet to use inside the workbook (e.g., "Malla 2020")
	pub sheet: Option<String>,

	// Optional: ranking académico del alumno expresado como percentil (0.0 - 1.0)
	pub student_ranking: Option<f64>,

	// Optional ranking/preferences provided by the user (may be absent)
	pub ranking: Option<Vec<String>>,

	// Optional: umbral para filtrar soluciones por dificultad.
	// Si se proporciona, se interpreta como un valor 0.0-1.0. Para cada
	// solución se calcula el producto de las probabilidades de reprobar
	// (1 - pct_aprobados/100) para cada ramo; si el producto > umbral,
	// la solución se descarta.
	/// Filtros opcionales del usuario (Reglas 3-6 del Plan).
	/// Cada filtro puede estar habilitado o deshabilitado independientemente.
	/// Si está deshabilitado, se ignora completamente.
	#[serde(default)]
	pub filtros: Option<UserFilters>,
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
    let params = parse_json_input(json_str)?;

    // delegar la lógica de resolución a la función que acepta InputParams
    resolve_ramos_with_resolver(params, base_dir, resolver)
}

/// Resolver ramos de un InputParams ya parseado (inyección de resolver para tests)
pub fn resolve_ramos_with_resolver<P, F>(mut params: InputParams, base_dir: Option<P>, resolver: F) -> Result<InputParams, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
    F: Fn(&Path, &str) -> Result<Option<String>, Box<dyn std::error::Error>>,
{
    let malla_name = params.malla.clone();
    let malla_path: PathBuf = match base_dir {
        Some(b) => b.as_ref().join(malla_name.clone()),
        None => PathBuf::from(malla_name.clone()),
    };

    // heurística simple: si la cadena contiene un dígito la consideramos código
    fn looks_like_code(s: &str) -> bool {
        s.chars().any(|c| c.is_ascii_digit())
    }

    let resolve_one = |r: String| -> String {
        if looks_like_code(&r) { return r; }
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
	fn test_parse_json_with_filtros() {
		// JSON completo con todos los filtros opcionales
		let json_data = r#"
		{
			"email": "estudiante@example.com",
			"ramos_pasados": ["CBM1000", "CBM1001"],
			"ramos_prioritarios": ["CIT3313"],
			"horarios_preferidos": ["08:00-10:00"],
			"malla": "MiMalla.xlsx",
			"sheet": null,
			"student_ranking": 0.75,
			"ranking": null,
			"filtros": {
				"dias_horarios_libres": {
					"habilitado": false,
					"dias_libres_preferidos": ["VI"],
					"minimizar_ventanas": true,
					"ventana_ideal_minutos": 30
				},
				"ventana_entre_actividades": {
					"habilitado": true,
					"minutos_entre_clases": 15
				},
				"preferencias_profesores": {
					"habilitado": false,
					"profesores_preferidos": ["Dr. García"],
					"profesores_evitar": []
				},
				"balance_lineas": {
					"habilitado": false,
					"lineas": {
						"informatica": 0.6,
						"telecomunicaciones": 0.4
					}
				}
			}
		}
		"#;

		let params = parse_json_input(json_data).expect("Debe parsear JSON con filtros");
		
		// Validaciones básicas
		assert_eq!(params.email, "estudiante@example.com");
		assert_eq!(params.ramos_pasados, vec!["CBM1000", "CBM1001"]);
		assert_eq!(params.malla, "MiMalla.xlsx");
		assert_eq!(params.student_ranking, Some(0.75));

		// Validar que filtros se deserializaron correctamente
		let filtros = params.filtros.expect("Debe haber filtros");
		
		// Comprobar ventana_entre_actividades está habilitado
		let ventana = filtros.ventana_entre_actividades.expect("Debe haber ventana_entre_actividades");
		assert!(ventana.habilitado);
		assert_eq!(ventana.minutos_entre_clases, Some(15));

		// Comprobar dias_horarios_libres está deshabilitado
		let dias = filtros.dias_horarios_libres.expect("Debe haber dias_horarios_libres");
		assert!(!dias.habilitado);
		assert_eq!(dias.dias_libres_preferidos, Some(vec!["VI".to_string()]));

		// Comprobar preferencias_profesores
		let profs = filtros.preferencias_profesores.expect("Debe haber preferencias_profesores");
		assert!(!profs.habilitado);
		assert_eq!(profs.profesores_preferidos, Some(vec!["Dr. García".to_string()]));

		// Comprobar balance_lineas
		let balance = filtros.balance_lineas.expect("Debe haber balance_lineas");
		assert!(!balance.habilitado);
		let lineas = balance.lineas.expect("Debe haber lineas map");
		assert_eq!(lineas.get("informatica"), Some(&0.6));
		assert_eq!(lineas.get("telecomunicaciones"), Some(&0.4));
	}

	#[test]
	fn test_parse_json_sin_filtros() {
		// JSON sin filtros (backward compatible)
		let json_data = r#"
		{
			"email": "alumno@ejemplo.cl",
			"ramos_pasados": ["CIT3313", "CIT3211"],
			"ramos_prioritarios": ["CIT3313", "CIT3413"],
			"horarios_preferidos": ["08:00-10:00", "14:00-16:00"],
			"malla": "MallaCurricular2020.xlsx"
		}
		"#;

		let params = parse_json_input(json_data).expect("Debe parsear JSON sin filtros");
		assert_eq!(params.ramos_pasados, vec!["CIT3313", "CIT3211"]);
		assert_eq!(params.ramos_prioritarios, vec!["CIT3313", "CIT3413"]);
		assert_eq!(params.horarios_preferidos, vec!["08:00-10:00", "14:00-16:00"]);
		assert_eq!(params.malla, "MallaCurricular2020.xlsx");
		
		// filtros debe estar None
		assert!(params.filtros.is_none());
	}

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
}
