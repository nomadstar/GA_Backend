/// Test con la consulta específica del usuario
use quickshift::api_json::parse_and_resolve_ramos;
use quickshift::algorithm::ruta::ejecutar_ruta_critica_with_params;
use std::env;
use std::fs;

#[test]
fn test_specific_user_query() {
    // Cambiar al directorio correcto
    if let Ok(cwd) = env::current_dir() {
        let cwd_str = cwd.to_string_lossy();
        if !cwd_str.contains("quickshift") {
            let _ = env::set_current_dir("/home/ignatus/GitHub/GA_Backend/quickshift");
        }
    }
    
    eprintln!("\n🔬 TEST: Consulta específica del usuario");
    eprintln!("==========================================\n");
    
    // Leer el JSON de prueba
    let request_json = fs::read_to_string("test_query.json")
        .expect("No se pudo leer test_query.json");
    
    eprintln!("📋 Query:");
    eprintln!("  - Malla: MC2020moded.xlsx");
    eprintln!("  - Ramos pasados: 15 cursos (incluyendo CFG1, CIT2006, CIT2114)");
    eprintln!("  - Filtros: minimize-gaps, ventana 15 min");
    eprintln!();
    
    // Parsear parámetros
    let params = match parse_and_resolve_ramos(&request_json, Some(".")) {
        Ok(p) => {
            eprintln!("✅ Parámetros parseados exitosamente");
            eprintln!("  - Ramos pasados: {:?}", p.ramos_pasados);
            eprintln!();
            p
        },
        Err(e) => {
            eprintln!("❌ Error al parsear: {}", e);
            panic!("Failed to parse: {}", e);
        }
    };
    
    // Ejecutar búsqueda
    eprintln!("🔍 Ejecutando búsqueda de soluciones...\n");
    let soluciones = match ejecutar_ruta_critica_with_params(params) {
        Ok(sols) => {
            eprintln!("✅ Búsqueda completada");
            sols
        },
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            panic!("Failed: {}", e);
        }
    };
    
    eprintln!("\n📈 RESULTADOS:");
    eprintln!("  Total soluciones: {}", soluciones.len());
    eprintln!();
    
    // Analizar por tamaño
    let mut by_size: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (sol, _) in &soluciones {
        *by_size.entry(sol.len()).or_insert(0) += 1;
    }
    
    eprintln!("📊 Distribución por tamaño:");
    for size in 1..=7 {
        if let Some(count) = by_size.get(&size) {
            eprintln!("  {} cursos: {} soluciones", size, count);
        }
    }
    eprintln!();
    
    // Mostrar top 10
    eprintln!("🔝 Top 10 soluciones:");
    for (idx, (sol, score)) in soluciones.iter().take(10).enumerate() {
        eprintln!("\n  Sol #{} - {} cursos (score: {})", idx + 1, sol.len(), score);
        eprintln!("  Cursos:");
        for (sec, pri) in sol {
            eprintln!("    - {} {} (sec: {}, pri: {})", 
                sec.codigo, sec.nombre, sec.seccion, pri);
        }
    }
    
    eprintln!("\n📋 DIAGNÓSTICO:");
    let seis_cursos = soluciones.iter().filter(|(s, _)| s.len() == 6).count();
    let cinco_cursos = soluciones.iter().filter(|(s, _)| s.len() == 5).count();
    
    eprintln!("  - Soluciones de 6 cursos: {}", seis_cursos);
    eprintln!("  - Soluciones de 5 cursos: {}", cinco_cursos);
    
    if seis_cursos < 10 {
        eprintln!("\n⚠️  PROBLEMA DETECTADO:");
        eprintln!("  El sistema no está generando suficientes soluciones de 6 cursos.");
        eprintln!("  Se esperaban al menos 10 soluciones de 6 cursos.");
    }
    
    eprintln!();
}
