use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;

#[test]
fn test_malla2020_con_calculo_i_aprobado() {
    println!("\n🔍 TEST: Malla2020.xlsx - CON Cálculo I aprobado");
    println!("{}", "=".repeat(80));

    // Usar EXACTAMENTE Malla2020.xlsx con CBM1001 aprobado
    let params = InputParams {
        email: "estudiante@example.com".to_string(),
        ramos_pasados: vec!["CBM1001".to_string()], // ✅ Cálculo I
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        malla: "Malla2020.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
    };

    println!("\n📋 Parámetros:");
    println!("   - malla: 'Malla2020.xlsx'");
    println!("   - ramos_pasados: ['CBM1001'] (Cálculo I)");

    match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            println!("\n✅ Soluciones generadas: {}", soluciones.len());

            if !soluciones.is_empty() {
                let (primer_sol, score) = &soluciones[0];
                println!("\n📌 Primera solución (score: {}):", score);
                println!("   Cursos recomendados ({} cursos):", primer_sol.len());

                let mut tiene_cbf1000 = false;
                let mut tiene_cbm1003 = false;

                for (sec, _priority) in primer_sol {
                    println!("     - {} ({})", sec.codigo, sec.nombre);
                    if sec.codigo == "CBF1000" {
                        tiene_cbf1000 = true;
                    }
                    if sec.codigo == "CBM1003" {
                        tiene_cbm1003 = true;
                    }
                }

                println!("\n📊 ANÁLISIS:");
                if tiene_cbf1000 {
                    println!("   ✅ CBF1000 (Mecánica) aparece - Correcto (requiere CBM1001 ✓)");
                } else {
                    println!("   ⚠️  CBF1000 (Mecánica) NO aparece");
                }

                if tiene_cbm1003 {
                    println!("   ✅ CBM1003 (Cálculo II) aparece - Correcto (requiere CBM1001 ✓)");
                } else {
                    println!("   ⚠️  CBM1003 (Cálculo II) NO aparece");
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
fn test_malla2020_con_primer_semestre_completo() {
    println!("\n🔍 TEST: Malla2020.xlsx - CON Semestre 1 completo");
    println!("{}", "=".repeat(80));

    // Semestre 1 completo
    let params = InputParams {
        email: "estudiante@example.com".to_string(),
        ramos_pasados: vec![
            "CBM1000".to_string(), // Álgebra
            "CBM1001".to_string(), // Cálculo I
            "CBQ1000".to_string(), // Química
            "CIT1000".to_string(), // Programación
            "FIC1000".to_string(), // Comunicación
        ],
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        malla: "Malla2020.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
    };

    println!("\n📋 Parámetros:");
    println!("   - malla: 'Malla2020.xlsx'");
    println!("   - ramos_pasados: 5 (Semestre 1 completo)");

    match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            println!("\n✅ Soluciones generadas: {}", soluciones.len());

            if !soluciones.is_empty() {
                let (primer_sol, score) = &soluciones[0];
                println!("\n📌 Primera solución (score: {}):", score);
                println!("   Cursos recomendados ({} cursos):", primer_sol.len());

                let mut cursos_sem2 = Vec::new();
                for (sec, _priority) in primer_sol {
                    println!("     - {} ({})", sec.codigo, sec.nombre);
                    cursos_sem2.push(sec.codigo.clone());
                }

                println!("\n📊 ANÁLISIS:");
                println!("   Deberían ser principalmente cursos de Semestre 2:");
                if cursos_sem2.contains(&"CBM1003".to_string()) || 
                   cursos_sem2.contains(&"CBF1000".to_string()) ||
                   cursos_sem2.contains(&"CBM1002".to_string()) {
                    println!("   ✅ Hay cursos de Semestre 2");
                }
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            panic!("Error ejecutando ruta crítica: {}", e);
        }
    }
}
