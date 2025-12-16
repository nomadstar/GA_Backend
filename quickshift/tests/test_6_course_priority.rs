/// Test para verificar que el sistema prioriza soluciones de 6 cursos
/// y que las primeras 10 soluciones tengan exactamente 6 cursos cada una
use quickshift::api_json::parse_and_resolve_ramos;
use quickshift::algorithm::ruta::ejecutar_ruta_critica_with_params;
use serde_json::json;
use std::env;

#[test]
fn test_6_course_solutions_priority() {
    // Cambiar al directorio correcto si es necesario
    if let Ok(cwd) = env::current_dir() {
        let cwd_str = cwd.to_string_lossy();
        if !cwd_str.contains("quickshift") {
            let _ = env::set_current_dir("/home/ignatus/GitHub/GA_Backend/quickshift");
        }
    }
    
    eprintln!("\n🔬 TEST: Verificar que las primeras 10 soluciones tengan 6 cursos");
    eprintln!("===================================================================\n");
    
    // Request JSON similar al de Python (semestre 3)
    let request_json = json!({
        "malla": "MC2020.xlsx",
        "ramos_pasados": [
            "CBM1000", "CBM1001", "CBQ1000", "CIG1012",  // Sem 1
            "CBM1002", "CBF1003", "CIM1010", "CIG1013"   // Sem 2
        ],
        "ramos_prioritarios": [],
        "horarios_preferidos": [],
        "horarios_prohibidos": [],
        "email": "test@example.com"
    }).to_string();
    
    eprintln!("📋 Request JSON (simulando estudiante terminando semestre 2):\n");
    eprintln!("  Ramos pasados (8):");
    eprintln!("    - Sem 1: CBM1000, CBM1001, CBQ1000, CIG1012 (Inglés I)");
    eprintln!("    - Sem 2: CBM1002, CBF1003, CIM1010, CIG1013 (Inglés II)");
    eprintln!();
    
    // Parsear y resolver
    let params = match parse_and_resolve_ramos(&request_json, Some(".")) {
        Ok(p) => p,
        Err(e) => panic!("Failed to parse parameters: {}", e)
    };
    
    // Ejecutar búsqueda
    let soluciones = match ejecutar_ruta_critica_with_params(params) {
        Ok(sols) => sols,
        Err(e) => panic!("Failed to execute ruta critica: {}", e)
    };
    
    eprintln!("📈 Resultados:");
    eprintln!("  Total de soluciones: {}\n", soluciones.len());
    
    // Analizar distribución por tamaño
    let mut by_size: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (sol, _) in &soluciones {
        *by_size.entry(sol.len()).or_insert(0) += 1;
    }
    
    eprintln!("📊 Distribución por tamaño:");
    for size in 1..=6 {
        if let Some(count) = by_size.get(&size) {
            eprintln!("  {} cursos: {} soluciones", size, count);
        }
    }
    eprintln!();
    
    // Mostrar primeras 10 soluciones
    eprintln!("🔝 Primeras 10 soluciones:");
    let mut all_6_courses = true;
    for (idx, (sol, score)) in soluciones.iter().take(10).enumerate() {
        let num_courses = sol.len();
        let icon = if num_courses == 6 { "✅" } else { "❌" };
        eprintln!("  {} Sol #{}: {} cursos (score: {})", icon, idx + 1, num_courses, score);
        
        if num_courses != 6 {
            all_6_courses = false;
            // Mostrar cuáles cursos tiene
            eprintln!("      Cursos: {}", 
                sol.iter()
                    .map(|(s, _)| s.codigo.as_str())
                    .collect::<Vec<_>>()
                    .join(", "));
        }
    }
    eprintln!();
    
    // Comparación con Python
    eprintln!("📊 Comparación con Python RutaCritica:");
    eprintln!("  Python:     10/10 soluciones con 6 cursos (100%)");
    eprintln!("  Quickshift: {}/10 soluciones con 6 cursos ({}%)", 
              soluciones.iter().take(10).filter(|(s, _)| s.len() == 6).count(),
              (soluciones.iter().take(10).filter(|(s, _)| s.len() == 6).count() * 100) / 10);
    eprintln!();
    
    // ASSERTION 1: Debe haber al menos 10 soluciones
    assert!(
        soluciones.len() >= 10,
        "FALLÓ: Solo {} soluciones encontradas, se esperaban al menos 10",
        soluciones.len()
    );
    
    // ASSERTION 2: Las primeras 10 deben ser de 6 cursos
    if !all_6_courses {
        eprintln!("❌ FALLÓ: No todas las primeras 10 soluciones tienen 6 cursos");
        eprintln!("   Esto indica que el algoritmo no está priorizando correctamente");
        eprintln!("   las soluciones de 6 cursos sobre las de 5 cursos.\n");
    }
    
    assert!(
        all_6_courses,
        "FALLÓ: Las primeras 10 soluciones deben tener exactamente 6 cursos cada una para igualar la calidad de Python"
    );
    
    eprintln!("✅ TEST PASÓ: Las primeras 10 soluciones tienen 6 cursos cada una");
    eprintln!("   Quickshift ahora iguala la calidad de Python RutaCritica ✨\n");
}
