/// Test para verificar que el sistema entrega mínimo 10 soluciones
use quickshift::api_json::parse_and_resolve_ramos;
use quickshift::algorithm::ruta::ejecutar_ruta_critica_with_params;
use serde_json::json;
use std::env;

#[test]
fn test_minimum_10_solutions() {
    // Cambiar al directorio correcto si es necesario
    if let Ok(cwd) = env::current_dir() {
        let cwd_str = cwd.to_string_lossy();
        if !cwd_str.contains("quickshift") {
            // Si no estamos en quickshift, intentar cambiar
            let _ = env::set_current_dir("/home/ignatus/GitHub/GA_Backend/quickshift");
        }
    }
    eprintln!("\n🔬 TEST: Verificar que el sistema entrega mínimo 10 soluciones");
    eprintln!("=============================================================\n");
    
    // Crear un request JSON típico
    let request_json = json!({
        "malla": "MC2020.xlsx",
        "ramos_pasados": ["CBM1000", "CBM1001", "CBQ1000"],
        "ramos_prioritarios": ["CIT1010", "CBM1002"],
        "horarios_preferidos": [],
        "horarios_prohibidos": [],
        "email": "test@example.com"
    }).to_string();
    
    eprintln!("📋 Request JSON:\n{}\n", request_json);
    
    // Parsear y resolver
    let params = match parse_and_resolve_ramos(&request_json, Some(".")) {
        Ok(p) => {
            eprintln!("✅ Parámetros parseados exitosamente");
            p
        },
        Err(e) => {
            eprintln!("❌ Error al parsear parámetros: {}", e);
            panic!("Failed to parse parameters: {}", e);
        }
    };
    
    eprintln!("📊 Parámetros resueltos:");
    eprintln!("  - Ramos pasados: {} cursos", params.ramos_pasados.len());
    eprintln!("  - Ramos prioritarios: {} cursos", params.ramos_prioritarios.len());
    eprintln!("  - Malla: {}\n", params.malla);
    
    // Ejecutar la búsqueda de soluciones
    let soluciones = match ejecutar_ruta_critica_with_params(params) {
        Ok(sols) => {
            eprintln!("✅ Búsqueda completada exitosamente");
            sols
        },
        Err(e) => {
            eprintln!("❌ Error en búsqueda: {}", e);
            panic!("Failed to execute ruta critica: {}", e);
        }
    };
    
    eprintln!("\n📈 Resultados:");
    eprintln!("  Total de soluciones encontradas: {}", soluciones.len());
    
    // Mostrar primeras 10 soluciones
    for (idx, (sol, score)) in soluciones.iter().take(10).enumerate() {
        eprintln!("  {}. Score: {}, Cursos: {}", idx + 1, score, sol.len());
    }
    
    if soluciones.len() > 10 {
        eprintln!("  ... y {} soluciones más", soluciones.len() - 10);
    }
    
    // ASSERTION: Verificar que hay al menos 10 soluciones
    eprintln!("\n🔍 Verificación:");
    eprintln!("  ✓ Esperado: Mínimo 10 soluciones");
    eprintln!("  ✓ Obtenido: {} soluciones", soluciones.len());
    
    assert!(
        soluciones.len() >= 10,
        "FALLÓ: El sistema devolvió {} soluciones en lugar del mínimo de 10",
        soluciones.len()
    );
    
    eprintln!("\n✅ TEST PASÓ: El sistema entrega al menos 10 soluciones\n");
}
