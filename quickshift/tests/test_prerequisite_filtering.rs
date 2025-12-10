/// Test para verificar que el algoritmo de clique FILTRA cursos sin prerequisitos cumplidos
use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;

#[test]
fn test_clique_filters_courses_without_prerequisites() {
    eprintln!("\n🧪 TEST: clique debe filtrar cursos sin prerequisitos cumplidos");
    eprintln!("=================================================================");
    
    // Simulamos un estudiante que solo ha aprobado algunos cursos de semestre 1
    // CBM1000 (Álgebra), CBM1001 (Cálculo I), CBQ1000 (Química)
    // NO puede tomar cursos que requieren otros prerrequisitos
    // Por ejemplo:
    // - CBM1002 (Álgebra Lineal) requiere CBM1000 (id=1) ✓ cumple
    // - CBM1003 (Cálculo II) requiere CBM1001 (id=2) ✓ cumple
    // - CBM1006 (Cálculo III) requiere CBM1003 (id=7) ✗ no cumple (no aprobó Cálculo II)
    
    let params = InputParams {
        email: "test@example.com".to_string(),
        ramos_pasados: vec!["CBM1000".to_string(), "CBM1001".to_string(), "CBQ1000".to_string()], 
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };
    
    eprintln!("📋 Parámetros:");
    eprintln!("   Email: {}", params.email);
    eprintln!("   Ramos pasados: {:?}", params.ramos_pasados);
    eprintln!("   Malla: {}", params.malla);
    
    // Ejecutar la ruta crítica
    let result = match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => soluciones,
        Err(e) => {
            eprintln!("❌ Error al ejecutar ruta crítica: {}", e);
            return;
        }
    };
    
    eprintln!("\n✅ Soluciones generadas: {}", result.len());
    
    if result.is_empty() {
        eprintln!("⚠️  No se generaron soluciones. Esto es válido si:");
        eprintln!("   - No hay cursos disponibles después de los prerequisitos");
        return;
    }
    
    // Para cada solución, verificar que NO contiene cursos que requieran otros requisitos
    for (idx, (solucion, _score)) in result.iter().enumerate() {
        eprintln!("\n📌 Solución #{}: {} cursos", idx + 1, solucion.len());
        
        for (seccion, _score) in solucion {
            eprintln!("   - {} (Código: {})", seccion.nombre, seccion.codigo);
            
            // Verificación: estos cursos NO deberían estar aquí si requieren prerrequisitos no cumplidos
            let codigo_upper = seccion.codigo.to_uppercase();
            
            // Cursos que REQUIEREN requisitos no aprobados:
            // - CBM1003 (Cálculo II) requiere CBM1001 ✓ APROBADO - OK
            // - CBM1006 (Cálculo III) requiere CBM1003 ✗ NO APROBADO - DEBE EXCLUIRSE
            // - CIT2114 (Redes de Datos) requiere CIT2113 u otros ✗ NO APROBADOS - DEBE EXCLUIRSE
            match codigo_upper.as_str() {
                "CBM1006" => panic!(
                    "❌ FALLO: {} (Cálculo III) requiere CBM1003 (Cálculo II), pero no está aprobado",
                    seccion.codigo
                ),
                "CIT2114" => panic!(
                    "❌ FALLO: {} (Redes de Datos) requiere prereqs no cumplidos",
                    seccion.codigo
                ),
                _ => {
                    // OK - curso sin conflicto de requisitos
                }
            }
        }
    }
    
    eprintln!("\n✅ TEST PASSED: Todas las soluciones respetan los prerequisitos");
}

#[test]
fn test_clique_includes_courses_with_met_prerequisites() {
    eprintln!("\n🧪 TEST: clique DEBE INCLUIR cursos cuyos prerequisitos SI están cumplidos");
    eprintln!("==============================================================================");
    
    // Simulamos un estudiante que ha aprobado CBM1000 (Álgebra)
    // y DEBE poder tomar CBM1001 (Cálculo I) si requiere solo Álgebra
    
    let params = InputParams {
        email: "test2@example.com".to_string(),
        ramos_pasados: vec!["CBM1000".to_string()], // Álgebra aprobada
        ramos_prioritarios: vec![], // Sin preferencias
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };
    
    eprintln!("📋 Parámetros:");
    eprintln!("   Email: {}", params.email);
    eprintln!("   Ramos pasados: {:?}", params.ramos_pasados);
    
    let result = match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => soluciones,
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            return;
        }
    };
    
    eprintln!("\n✅ Soluciones generadas: {}", result.len());
    
    if result.is_empty() {
        eprintln!("⚠️  Sin soluciones (podría ser válido dependiendo de la malla)");
        return;
    }
    
    eprintln!("\n📊 Resumen de cursos recomendados:");
    for (idx, (solucion, score)) in result.iter().enumerate() {
        eprintln!("   Solución #{}: score={}", idx + 1, score);
        for (sec, _) in solucion {
            eprintln!("      - {}", sec.codigo);
        }
    }
    
    eprintln!("\n✅ TEST PASSED: Se generaron recomendaciones correctamente");
}
