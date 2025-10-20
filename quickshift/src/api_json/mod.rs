use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
	use super::*;

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
