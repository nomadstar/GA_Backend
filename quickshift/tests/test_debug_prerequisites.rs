use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;

#[test]
fn test_debug_sin_ramos_aprobados() {
    println!("\n🔍 TEST: Debug - Sin ramos aprobados");
    println!("{}", "=".repeat(80));

    // Crear parámetros SIN ramos aprobados
    let params = InputParams {
        email: "test@example.com".to_string(),
        ramos_pasados: vec![], // ❌ VACÍO
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
    };

    println!("\n📋 Parámetros:");
    println!("   - ramos_pasados: {} (VACÍO)", params.ramos_pasados.len());
    println!("   - Esperamos: Solo cursos de Semestre 1 sin requisitos");

    match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            println!("\n✅ Soluciones generadas: {}", soluciones.len());

            if !soluciones.is_empty() {
                let (primer_sol, score) = &soluciones[0];
                println!("\n📌 Primera solución (score: {}):", score);
                println!("   Cursos recomendados:");

                let mut tiene_mecanica = false;
                for (sec, priority) in primer_sol {
                    println!("     - {} ({})", sec.codigo, sec.nombre);

                    if sec.codigo == "CBF1000" {
                        tiene_mecanica = true;
                        println!("       ⚠️  ALERTA: CBF1000 (Mecánica) NO DEBERÍA ESTAR");
                        println!("           - Mecánica requiere CBM1001 (Cálculo I)");
                        println!("           - CBM1001 NO está aprobado");
                    }

                    if sec.codigo == "CBM1001" {
                        println!("       ✅ CORRECTO: CBM1001 (Cálculo I) - Semestre 1, sin requisitos");
                    }
                }

                if tiene_mecanica {
                    println!("\n❌ TEST FAILED: CBF1000 NO debería recomendarse sin CBM1001");
                    panic!("Prerequisito no validado correctamente");
                } else {
                    println!("\n✅ TEST PASSED: Prerequisitos validados correctamente");
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            panic!("Error ejecutando ruta crítica: {}", e);
        }
    }
}

#[test]
fn test_debug_con_calculo_i() {
    println!("\n🔍 TEST: Debug - Con Cálculo I (CBM1001) aprobado");
    println!("{}", "=".repeat(80));

    // Crear parámetros CON Cálculo I aprobado
    let params = InputParams {
        email: "test@example.com".to_string(),
        ramos_pasados: vec!["CBM1001".to_string()], // ✅ Cálculo I
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
    };

    println!("\n📋 Parámetros:");
    println!("   - ramos_pasados: {} (CBM1001)", params.ramos_pasados.len());
    println!("   - Esperamos: Cursos de Semestre 2+ que requieran CBM1001");

    match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            println!("\n✅ Soluciones generadas: {}", soluciones.len());

            if !soluciones.is_empty() {
                let (primer_sol, score) = &soluciones[0];
                println!("\n📌 Primera solución (score: {}):", score);
                println!("   Cursos recomendados:");

                let mut tiene_mecanica = false;
                for (sec, _priority) in primer_sol {
                    println!("     - {} ({})", sec.codigo, sec.nombre);

                    if sec.codigo == "CBF1000" {
                        tiene_mecanica = true;
                        println!("       ✅ CORRECTO: CBF1000 (Mecánica) - Requiere CBM1001 ✓");
                    }
                }

                if tiene_mecanica {
                    println!("\n✅ TEST PASSED: CBF1000 aparece cuando CBM1001 está aprobado");
                } else {
                    println!("\n⚠️  TEST: CBF1000 no está en la solución (puede ser válido)");
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            panic!("Error ejecutando ruta crítica: {}", e);
        }
    }
}
