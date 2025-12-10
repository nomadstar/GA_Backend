use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;
use std::collections::{HashMap, HashSet};

#[test]
fn test_solutions_have_section_diversity() {
    eprintln!("\n🧪 TEST: Verificar diversidad de `codigo_box` entre soluciones");

    let params = InputParams {
        email: "diversity@test".to_string(),
        ramos_pasados: vec![],
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        malla: "MC2020.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.5),
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };

    let soluciones = match ejecutar_ruta_critica_with_params(params) {
        Ok(s) => s,
        Err(e) => panic!("Error ejecutando ruta critica: {}", e),
    };

    assert!(soluciones.len() >= 2, "Se esperaban al menos 2 soluciones para probar diversidad, encontradas: {}", soluciones.len());

    // Map: codigo -> set de codigo_box observados en todas las soluciones
    let mut map: HashMap<String, HashSet<String>> = HashMap::new();

    for (sol, _score) in soluciones.iter() {
        for (sec, _pref) in sol.iter() {
            map.entry(sec.codigo.clone()).or_default().insert(sec.codigo_box.clone());
        }
    }

    // Buscar al menos un ramo con más de 1 codigo_box distinto
    let mut found = false;
    for (codigo, set) in map.iter() {
        if set.len() > 1 {
            eprintln!("✅ Curso {} tiene {} variantes de codigo_box", codigo, set.len());
            found = true;
            break;
        }
    }

    if !found {
        eprintln!("No se detectó diversidad de secciones entre las soluciones. Conteos:");
        for (codigo, set) in map.iter() {
            eprintln!("  - {} -> {}", codigo, set.len());
        }
    }

    assert!(found, "Todas las soluciones usan la misma `codigo_box` por curso; falta diversidad entre secciones.");
}
