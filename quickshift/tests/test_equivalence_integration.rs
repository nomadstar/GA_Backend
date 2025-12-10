use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;

#[test]
fn test_equivalence_integration_full_pipeline() {
    // Test completo que verifica que al pasar CIG1014 (INGLÉS GENERAL III),
    // el sistema NO propone CIG1003 (que es el equivalente)
    
    let params = InputParams {
        email: "test@example.com".to_string(),
        malla: "MC2020moded.xlsx".to_string(),
        ramos_pasados: vec![
            "CIG1014".to_string(), // INGLÉS GENERAL III (del sistema OA20251)
            "MAT1610".to_string(), // Cálculo I
            "CIT1001".to_string(), // Programación I
        ],
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        sheet: None,
        anio: Some(2020),
        student_ranking: None,
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };
    
    eprintln!("\n=== TEST: Equivalencia CIG1014 -> CIG1003 ===");
    eprintln!("Ramos pasados ANTES de mapeo: {:?}", params.ramos_pasados);
    
    match ejecutar_ruta_critica_with_params(params) {
        Ok(solutions) => {
            eprintln!("\n✅ Pipeline ejecutado exitosamente");
            eprintln!("   Número de soluciones: {}", solutions.len());
            
            // Verificar que ninguna solución contiene CIG1003
            // (porque ya pasó CIG1014 que es equivalente)
            let mut found_cig1003 = false;
            for (idx, (secciones, score)) in solutions.iter().enumerate() {
                eprintln!("\nSolución {}: Score={}", idx + 1, score);
                for (seccion, _priority) in secciones {
                    eprintln!("  - {} ({})", seccion.codigo, seccion.nombre);
                    if seccion.codigo == "CIG1003" {
                        found_cig1003 = true;
                        eprintln!("     ⚠️  ENCONTRADO CIG1003 EN SOLUCIÓN!");
                    }
                }
            }
            
            if found_cig1003 {
                eprintln!("\n❌ FALLO: CIG1003 aparece en alguna solución");
                eprintln!("   El estudiante ya pasó CIG1014 (equivalente)");
                panic!("CIG1003 no debería aparecer si ya pasó CIG1014");
            } else {
                eprintln!("\n✅ ÉXITO: CIG1003 no aparece en ninguna solución");
                eprintln!("   El mapeo de equivalencias funciona correctamente");
            }
        }
        Err(e) => {
            eprintln!("❌ Error ejecutando pipeline: {}", e);
            panic!("Pipeline falló: {}", e);
        }
    }
}

#[test]
fn test_multiple_equivalences() {
    // Test verificando que múltiples equivalencias funcionan simultáneamente
    
    let params = InputParams {
        email: "test@example.com".to_string(),
        malla: "MC2020moded.xlsx".to_string(),
        ramos_pasados: vec![
            "CIG1014".to_string(), // INGLÉS GENERAL III -> CIG1003
            "CIG1012".to_string(), // INGLÉS GENERAL II -> CIG1002
        ],
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        sheet: None,
        anio: Some(2020),
        student_ranking: None,
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };
    
    eprintln!("\n=== TEST: Múltiples equivalencias ===");
    eprintln!("Ramos pasados (con equivalencias): {:?}", params.ramos_pasados);
    
    match ejecutar_ruta_critica_with_params(params) {
        Ok(solutions) => {
            eprintln!("✅ Ejecutado con {} soluciones", solutions.len());
            
            // Verificar que no aparecen los equivalentes
            for (idx, (secciones, _score)) in solutions.iter().enumerate() {
                for (seccion, _) in secciones {
                    let codigo = &seccion.codigo;
                    assert_ne!(codigo, "CIG1003", "CIG1003 no debería aparecer");
                    assert_ne!(codigo, "CIG1002", "CIG1002 no debería aparecer");
                }
            }
            
            eprintln!("✅ Verificado: No aparecen equivalencias de cursos ya pasados");
        }
        Err(e) => {
            eprintln!("❌ Pipeline falló: {}", e);
            panic!("Error: {}", e);
        }
    }
}
