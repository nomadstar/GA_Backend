use quickshift::api_json::*;
use std::path::Path;

#[test]
fn test_parse_json_with_filtros() {
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
    assert_eq!(params.email, "estudiante@example.com");
    assert_eq!(params.ramos_pasados, vec!["CBM1000", "CBM1001"]);
    assert_eq!(params.malla, "MiMalla.xlsx");
    assert_eq!(params.student_ranking, Some(0.75));

    let filtros = params.filtros.expect("Debe haber filtros");
    let ventana = filtros.ventana_entre_actividades.expect("Debe haber ventana_entre_actividades");
    assert!(ventana.habilitado);
    assert_eq!(ventana.minutos_entre_clases, Some(15));

    let dias = filtros.dias_horarios_libres.expect("Debe haber dias_horarios_libres");
    assert!(!dias.habilitado);
    assert_eq!(dias.dias_libres_preferidos, Some(vec!["VI".to_string()]));

    let profs = filtros.preferencias_profesores.expect("Debe haber preferencias_profesores");
    assert!(!profs.habilitado);
    assert_eq!(profs.profesores_preferidos, Some(vec!["Dr. García".to_string()]));

    let balance = filtros.balance_lineas.expect("Debe haber balance_lineas");
    assert!(!balance.habilitado);
    let lineas = balance.lineas.expect("Debe haber lineas map");
    assert_eq!(lineas.get("informatica"), Some(&0.6));
    assert_eq!(lineas.get("telecomunicaciones"), Some(&0.4));
}

#[test]
fn test_parse_json_sin_filtros() {
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
    assert!(params.filtros.is_none());
}

#[test]
fn test_parse_and_resolve_ramos_with_mock() {
    let json_data = r#"
    {
        "email": "juan.perez@example.com",
        "ramos_pasados": ["Algebra y Geometría", "Calculo 1", "Programación"],
        "ramos_prioritarios": ["Programación Avanzada", "Calculo 2"],
        "horarios_preferidos": ["08:00-10:00"],
                "malla": "MallaCurricularTest.xlsx"
    }
    "#;

    let resolver = |_p: &Path, name: &str| -> Result<Option<String>, Box<dyn std::error::Error>> {
        let lower = name.to_lowercase();
        if lower.contains("programación avanzada") { return Ok(Some("CIT9999".to_string().into())); }
        if lower.contains("programación") { return Ok(Some("CIT1001".to_string().into())); }
        if lower.contains("algebra") { return Ok(Some("MAT1000".to_string().into())); }
        Ok(None)
    };

    let params = parse_and_resolve_ramos_with_resolver(json_data, Some("."), resolver).unwrap();
    assert!(params.ramos_pasados.contains(&"MAT1000".to_string()));
    assert!(params.ramos_pasados.contains(&"CIT1001".to_string()));
    assert!(params.ramos_prioritarios.contains(&"CIT9999".to_string()));
    assert!(params.ramos_pasados.contains(&"Calculo 1".to_string()));
}
