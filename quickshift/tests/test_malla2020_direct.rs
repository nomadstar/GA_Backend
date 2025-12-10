use quickshift::api_json::InputParams;
use quickshift::algorithm::ejecutar_ruta_critica_with_params;

#[test]
fn test_malla2020_sin_ramos_aprobados() {
    println!("\n🔍 TEST: Malla2020.xlsx - Sin ramos aprobados");
    println!("{}", "=".repeat(80));

    // Usar EXACTAMENTE Malla2020.xlsx como el usuario indicó
    let params = InputParams {
        email: "estudiante@example.com".to_string(),
        ramos_pasados: vec![], // ❌ VACÍO
        ramos_prioritarios: vec![],
        horarios_preferidos: vec![],
        horarios_prohibidos: vec![],
        malla: "Malla2020.xlsx".to_string(), // ⚠️ Malla2020, no MiMalla
        anio: None,
        sheet: None,
        student_ranking: Some(0.75),
        ranking: None,
        filtros: None,
        optimizations: vec![],
    };

    println!("\n📋 Parámetros:");
    println!("   - malla: 'Malla2020.xlsx'");
    println!("   - ramos_pasados: {} (VACÍO)", params.ramos_pasados.len());

    match ejecutar_ruta_critica_with_params(params) {
        Ok(soluciones) => {
            println!("\n✅ Soluciones generadas: {}", soluciones.len());

            if !soluciones.is_empty() {
                let (primer_sol, score) = &soluciones[0];
                println!("\n📌 Primera solución (score: {}):", score);
                println!("   Cursos recomendados ({} cursos):", primer_sol.len());

                for (sec, _priority) in primer_sol {
                    println!("     - {} ({})", sec.codigo, sec.nombre);
                }

                println!("\n📊 ANÁLISIS:");
                // Verificar si hay cursos sin requisitos en Semestre 1
                println!("   ¿Todos son de Semestre 1 o sin requisitos?");
            }
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            panic!("Error ejecutando ruta crítica: {}", e);
        }
    }
}
