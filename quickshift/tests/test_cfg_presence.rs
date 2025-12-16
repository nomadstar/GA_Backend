use quickshift::algorithm::ruta::ejecutar_ruta_critica_with_params;
use quickshift::api_json::InputParams;
use quickshift::models::{UserFilters, DiaHorariosLibres, VentanaEntreActividades, PreferenciasProfesores};

fn create_base_params(ramos_pasados: Vec<String>) -> InputParams {
    InputParams {
        email: "ignacio.marambio_z@mail.udp.cl".to_string(),
        ramos_pasados,
        ramos_prioritarios: Vec::new(),
        horarios_preferidos: Vec::new(),
        horarios_prohibidos: Vec::new(),
        malla: "MC2020moded.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.5),
        ranking: None,
        filtros: Some(UserFilters {
            dias_horarios_libres: Some(DiaHorariosLibres {
                habilitado: true,
                dias_libres_preferidos: None,
                minimizar_ventanas: Some(true),
                ventana_ideal_minutos: Some(30),
                franjas_prohibidas: None,
                no_sin_horario: None,
            }),
            ventana_entre_actividades: Some(VentanaEntreActividades {
                habilitado: true,
                minutos_entre_clases: Some(15),
            }),
            preferencias_profesores: Some(PreferenciasProfesores {
                habilitado: false,
                profesores_preferidos: None,
                profesores_evitar: None,
            }),
            balance_lineas: None,
        }),
        optimizations: vec!["minimize-gaps".to_string()],
    }
}

fn count_cfgs_in_passed(ramos_pasados: &[String]) -> usize {
    ramos_pasados.iter()
        .filter(|r| r.to_uppercase().starts_with("CFG"))
        .count()
}

fn count_cfgs_in_solution(sol: &[(quickshift::models::Seccion, i32)]) -> usize {
    sol.iter()
        .filter(|(sec, _)| sec.is_cfg && sec.codigo.to_uppercase().starts_with("CFG"))
        .count()
}

#[test]
fn test_cfg_basic_presence() {
    // Caso básico: 0 CFGs aprobados -> debe haber al menos 1 CFG en soluciones
    let params = create_base_params(vec![
        "CBM1000".to_string(),
        "CBM1001".to_string(),
        "CBQ1000".to_string(),
        "CIT1000".to_string(),
        "FIC1000".to_string(),
    ]);

    let res = ejecutar_ruta_critica_with_params(params);

    match res {
        Ok(sols) => {
            let found_cfg = sols.iter().any(|(sol, _)| {
                sol.iter().any(|(sec, _)| sec.is_cfg && sec.codigo.to_uppercase().starts_with("CFG"))
            });
            assert!(found_cfg, "ERROR: ninguna solución contiene un curso CFG (0 CFGs aprobados)");
        }
        Err(e) => panic!("ejecutar_ruta_critica_with_params devolvió error: {}", e),
    }
}

#[test]
fn test_cfg_with_one_approved() {
    // Caso: 1 CFG aprobado (CFG1) -> debe haber hasta 3 CFGs en soluciones
    let params = create_base_params(vec![
        "CBM1000".to_string(), "CBM1001".to_string(), "CBQ1000".to_string(),
        "CIT1000".to_string(), "FIC1000".to_string(), "CFG1".to_string(),
        "CIT1010".to_string(), "CBF1000".to_string(), "CBM1003".to_string(),
        "CBM1002".to_string(), "CBM1005".to_string(), "CBM1006".to_string(),
        "CBF1001".to_string(), "CIT2006".to_string(), "CIT2114".to_string(),
    ]);

    let cfgs_aprobados = count_cfgs_in_passed(&params.ramos_pasados);
    let max_cfgs_esperados = 4 - cfgs_aprobados;

    let res = ejecutar_ruta_critica_with_params(params);

    match res {
        Ok(sols) => {
            // Verificar que al menos una solución tiene CFGs
            let found_cfg = sols.iter().any(|(sol, _)| count_cfgs_in_solution(sol) > 0);
            assert!(found_cfg, "ERROR: ninguna solución contiene CFGs (1 CFG aprobado)");
            
            // Verificar que ninguna solución excede el máximo de CFGs permitidos
            for (sol, _) in sols.iter() {
                let cfg_count = count_cfgs_in_solution(sol);
                assert!(
                    cfg_count <= max_cfgs_esperados,
                    "Solución tiene {} CFGs, pero máximo esperado es {} (CFGs aprobados: {})",
                    cfg_count, max_cfgs_esperados, cfgs_aprobados
                );
            }
        }
        Err(e) => panic!("ejecutar_ruta_critica_with_params devolvió error: {}", e),
    }
}

#[test]
fn test_cfg_with_two_approved() {
    // Caso: 2 CFGs aprobados (CFG1, CFG2) -> debe haber hasta 2 CFGs en soluciones
    let params = create_base_params(vec![
        "CBM1000".to_string(), "CBM1001".to_string(), "CBQ1000".to_string(),
        "CIT1000".to_string(), "FIC1000".to_string(), "CFG1".to_string(),
        "CIT1010".to_string(), "CBF1000".to_string(), "CBM1003".to_string(),
        "CBM1002".to_string(), "CFG2".to_string(), "CIG1012".to_string(),
        "CIT2008".to_string(), "CIT2114".to_string(), "CIT2006".to_string(),
        "CBF1001".to_string(), "CBM1006".to_string(), "CBM1005".to_string(),
        "CIT2204".to_string(), "CIT2107".to_string(), "CBF1002".to_string(),
        "CIT2007".to_string(),
    ]);

    let cfgs_aprobados = count_cfgs_in_passed(&params.ramos_pasados);
    let max_cfgs_esperados = 4 - cfgs_aprobados;

    let res = ejecutar_ruta_critica_with_params(params);

    match res {
        Ok(sols) => {
            let found_cfg = sols.iter().any(|(sol, _)| count_cfgs_in_solution(sol) > 0);
            assert!(found_cfg, "ERROR: ninguna solución contiene CFGs (2 CFGs aprobados)");
            
            for (sol, _) in sols.iter() {
                let cfg_count = count_cfgs_in_solution(sol);
                assert!(
                    cfg_count <= max_cfgs_esperados,
                    "Solución tiene {} CFGs, pero máximo esperado es {} (CFGs aprobados: {})",
                    cfg_count, max_cfgs_esperados, cfgs_aprobados
                );
            }
        }
        Err(e) => panic!("ejecutar_ruta_critica_with_params devolvió error: {}", e),
    }
}

#[test]
fn test_cfg_with_three_approved() {
    // Caso: 3 CFGs aprobados (CFG1, CFG2, CFG3) -> debe haber hasta 1 CFG en soluciones
    let params = create_base_params(vec![
        "CBM1000".to_string(), "CBM1001".to_string(), "CBM1002".to_string(),
        "CBM1003".to_string(), "CBF1000".to_string(), "CBQ1000".to_string(),
        "CIT1000".to_string(), "FIC1000".to_string(), "CFG1".to_string(),
        "CIT1010".to_string(), "CIT2114".to_string(), "CIT2006".to_string(),
        "CBM1006".to_string(), "CBM1005".to_string(), "CIT2204".to_string(),
        "CIT2107".to_string(), "CBF1001".to_string(), "CBF1002".to_string(),
        "CIT2008".to_string(), "CIT2007".to_string(), "CFG2".to_string(),
        "CFG3".to_string(), "CIG1012".to_string(), "CIG1013".to_string(),
        "CIT2009".to_string(), "CIT2205".to_string(), "CIT2110".to_string(),
        "CIT2109".to_string(), "CIT2108".to_string(), "CII2750".to_string(),
        "CII1000".to_string(), "CIT2010".to_string(),
    ]);

    let cfgs_aprobados = count_cfgs_in_passed(&params.ramos_pasados);
    let max_cfgs_esperados = 4 - cfgs_aprobados;

    let res = ejecutar_ruta_critica_with_params(params);

    match res {
        Ok(sols) => {
            let found_cfg = sols.iter().any(|(sol, _)| count_cfgs_in_solution(sol) > 0);
            assert!(found_cfg, "ERROR: ninguna solución contiene CFGs (3 CFGs aprobados)");
            
            for (sol, _) in sols.iter() {
                let cfg_count = count_cfgs_in_solution(sol);
                assert!(
                    cfg_count <= max_cfgs_esperados,
                    "Solución tiene {} CFGs, pero máximo esperado es {} (CFGs aprobados: {})",
                    cfg_count, max_cfgs_esperados, cfgs_aprobados
                );
            }
        }
        Err(e) => panic!("ejecutar_ruta_critica_with_params devolvió error: {}", e),
    }
}

#[test]
fn test_cfg_all_approved() {
    // Caso límite: 4 CFGs aprobados -> NO debe haber CFGs en soluciones
    let params = create_base_params(vec![
        "CBM1000".to_string(), "CBM1001".to_string(), "CBM1002".to_string(),
        "CBM1003".to_string(), "CFG1".to_string(), "CFG2".to_string(),
        "CFG3".to_string(), "CFG4".to_string(), "CIT1000".to_string(),
        "CIT1010".to_string(), "FIC1000".to_string(), "CBF1000".to_string(),
    ]);

    let cfgs_aprobados = count_cfgs_in_passed(&params.ramos_pasados);
    
    // Validar que efectivamente hay 4 CFGs aprobados
    assert_eq!(cfgs_aprobados, 4, "Test mal configurado: se esperaban 4 CFGs aprobados, pero hay {}", cfgs_aprobados);

    let res = ejecutar_ruta_critica_with_params(params);

    match res {
        Ok(sols) => {
            for (sol, _) in sols.iter() {
                let cfg_count = count_cfgs_in_solution(sol);
                assert_eq!(
                    cfg_count, 0,
                    "Solución tiene {} CFGs, pero NO debería tener ninguno ({} CFGs ya aprobados)",
                    cfg_count, cfgs_aprobados
                );
            }
            println!("✓ Test pasado: {} CFGs aprobados → 0 CFGs en soluciones (correcto)", cfgs_aprobados);
        }
        Err(e) => panic!("ejecutar_ruta_critica_with_params devolvió error: {}", e),
    }
}
