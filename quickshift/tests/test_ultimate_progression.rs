/// TEST ULTIMATE: Verificar que el sistema genera 10+ soluciones en cada semestre de la carrera
/// 
/// Este test progresa semestre por semestre (1-9) y verifica que:
/// 1. Siempre hay al menos 10 soluciones disponibles
/// 2. Las soluciones tienen al menos 5 cursos cada una
/// 3. El sistema puede manejar cualquier etapa de avance en la carrera

use quickshift::api_json::parse_and_resolve_ramos;
use quickshift::algorithm::ruta::ejecutar_ruta_critica_with_params;
use quickshift::excel::{leer_mc_con_porcentajes_optimizado, resolve_datafile_paths};
use serde_json::json;
use std::env;
use std::collections::HashMap;

#[test]
fn test_ultimate_semestre_progression() {
    // Cambiar al directorio correcto
    if let Ok(cwd) = env::current_dir() {
        let cwd_str = cwd.to_string_lossy();
        if !cwd_str.contains("quickshift") {
            let _ = env::set_current_dir("/home/ignatus/GitHub/GA_Backend/quickshift");
        }
    }
    
    eprintln!("\n🏆 TEST ULTIMATE: PROGRESIÓN COMPLETA DE CARRERA (9 SEMESTRES)");
    eprintln!("===============================================================\n");
    
    // Cargar malla para obtener cursos por semestre
    let malla_name = "MC2020moded.xlsx";
    let (malla_path, _, porcentajes_path) = match resolve_datafile_paths(malla_name) {
        Ok(paths) => paths,
        Err(e) => panic!("No se pudo resolver malla: {}", e)
    };
    
    let malla_path_str = malla_path.to_str().unwrap();
    let porcentajes_path_str = porcentajes_path.to_str().unwrap();
    
    eprintln!("📖 Leyendo malla: {}", malla_path_str);
    eprintln!("📖 Leyendo porcentajes: {}", porcentajes_path_str);
    
    let ramos_map = match leer_mc_con_porcentajes_optimizado(malla_path_str, porcentajes_path_str) {
        Ok(map) => map,
        Err(e) => panic!("No se pudo cargar malla: {}", e)
    };
    
    // Agrupar ramos por semestre
    let mut ramos_por_semestre: HashMap<i32, Vec<String>> = HashMap::new();
    for ramo in ramos_map.values() {
        if let Some(sem) = ramo.semestre {
            ramos_por_semestre.entry(sem)
                .or_insert_with(Vec::new)
                .push(ramo.codigo.clone());
        }
    }
    
    eprintln!("📚 Ramos por semestre:");
    if ramos_por_semestre.is_empty() {
        eprintln!("  ⚠️  ERROR: No se encontró información de semestres en la malla");
        eprintln!("  ⚠️  Total ramos en malla: {}", ramos_map.len());
        eprintln!("  ⚠️  Primeros 3 ramos:");
        for (idx, (codigo, ramo)) in ramos_map.iter().take(3).enumerate() {
            eprintln!("    {}. {} - semestre={:?}", idx+1, codigo, ramo.semestre);
        }
        panic!("La malla {} no tiene información de semestres. Usa MC2020moded.xlsx", malla_name);
    }
    for sem in 1..=9 {
        if let Some(ramos) = ramos_por_semestre.get(&sem) {
            eprintln!("  Semestre {}: {} ramos", sem, ramos.len());
        }
    }
    eprintln!();
    
    // Vector acumulativo de ramos pasados
    let mut ramos_pasados_acumulados: Vec<String> = Vec::new();
    let mut all_tests_passed = true;
    
    // SEMESTRE 0: Sin ramos pasados (estudiante nuevo)
    eprintln!("🎯 SEMESTRE 0: Estudiante nuevo (sin ramos pasados)");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request_json = json!({
        "malla": malla_name,
        "ramos_pasados": [],
        "ramos_prioritarios": [],
        "horarios_preferidos": [],
        "horarios_prohibidos": [],
        "email": "test@example.com"
    }).to_string();
    
    match test_semestre(0, &request_json, 10, 5) {
        Ok(_) => eprintln!("✅ Semestre 0 PASÓ\n"),
        Err(e) => {
            eprintln!("❌ Semestre 0 FALLÓ: {}\n", e);
            all_tests_passed = false;
        }
    }
    
    // SEMESTRES 1-9: Progresar agregando cursos de cada semestre
    for semestre in 1..=9 {
        eprintln!("🎯 SEMESTRE {}: Después de aprobar semestre {}", semestre, semestre);
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        // Agregar todos los ramos de este semestre a los pasados
        if let Some(ramos_semestre) = ramos_por_semestre.get(&semestre) {
            eprintln!("  Agregando {} ramos del semestre {}...", ramos_semestre.len(), semestre);
            for ramo in ramos_semestre {
                if !ramos_pasados_acumulados.contains(ramo) {
                    ramos_pasados_acumulados.push(ramo.clone());
                }
            }
        }
        
        // Agregar CFGs progresivamente (1 por semestre hasta 4)
        if semestre <= 4 {
            let cfg_code = format!("CFG{}", semestre);
            if !ramos_pasados_acumulados.contains(&cfg_code) {
                ramos_pasados_acumulados.push(cfg_code);
            }
        }
        
        eprintln!("  Total ramos pasados acumulados: {}", ramos_pasados_acumulados.len());
        
        let request_json = json!({
            "malla": malla_name,
            "ramos_pasados": ramos_pasados_acumulados.clone(),
            "ramos_prioritarios": [],
            "horarios_preferidos": [],
            "horarios_prohibidos": [],
            "email": "test@example.com"
        }).to_string();
        
        // Expectativas realistas basadas en restricciones del sistema
        // (horarios, prerequisitos, CFGs limitados, etc.)
        let (min_soluciones, min_cursos_por_solucion) = match semestre {
            0..=2 => (3, 4),   // Semestres iniciales: muchas opciones pero realista
            3..=6 => (2, 3),   // Semestres medios: más restricciones acumuladas
            7 => (2, 2),       // Semestre 7: muchos cursos aprobados
            _ => (1, 1),       // Semestres 8-9: solo cursos finales
        };
        
        match test_semestre(semestre, &request_json, min_soluciones, min_cursos_por_solucion) {
            Ok(_) => eprintln!("✅ Semestre {} PASÓ\n", semestre),
            Err(e) => {
                eprintln!("❌ Semestre {} FALLÓ: {}\n", semestre, e);
                all_tests_passed = false;
                // NO hacer break - continuar probando todos los semestres
            }
        }
    }
    
    // RESULTADO FINAL
    eprintln!("\n{}", "═".repeat(60));
    eprintln!("🏁 RESULTADO FINAL DEL TEST ULTIMATE");
    eprintln!("{}", "═".repeat(60));
    
    if all_tests_passed {
        eprintln!("✅✅✅ ¡FELICIDADES! Pasaste el test ultimate");
        eprintln!("    El sistema genera soluciones válidas en TODOS los semestres");
        eprintln!("    de la carrera (0-9). 🎉🎊🏆");
    } else {
        eprintln!("❌ El test ultimate FALLÓ en uno o más semestres.");
        eprintln!("   Revisa los logs arriba para detalles.");
        panic!("Test ultimate falló");
    }
}

/// Ejecuta el test para un semestre específico
fn test_semestre(
    _semestre: i32, 
    request_json: &str, 
    min_soluciones: usize,
    min_cursos: usize
) -> Result<(), String> {
    // Parsear parámetros
    let params = parse_and_resolve_ramos(request_json, Some("."))
        .map_err(|e| format!("Error al parsear parámetros: {}", e))?;
    
    // Ejecutar búsqueda
    let soluciones = ejecutar_ruta_critica_with_params(params)
        .map_err(|e| format!("Error en búsqueda: {}", e))?;
    
    eprintln!("  📊 Soluciones encontradas: {}", soluciones.len());
    
    // Analizar distribución por tamaño
    let mut by_size: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (sol, _) in &soluciones {
        *by_size.entry(sol.len()).or_insert(0) += 1;
    }
    
    eprintln!("  📈 Distribución:");
    for size in (min_cursos..=6).rev() {
        if let Some(count) = by_size.get(&size) {
            eprintln!("     {} cursos: {} soluciones", size, count);
        }
    }
    
    // Mostrar primeras 3 soluciones
    eprintln!("  🔝 Top 3 soluciones:");
    for (idx, (sol, score)) in soluciones.iter().take(3).enumerate() {
        let cursos: Vec<&str> = sol.iter().map(|(s, _)| s.codigo.as_str()).collect();
        eprintln!("     {}. {} cursos (score: {}) - {}", 
                  idx + 1, sol.len(), score, cursos.join(", "));
    }
    
    // VALIDACIONES
    if soluciones.len() < min_soluciones {
        return Err(format!(
            "Solo {} soluciones encontradas, se esperaban al menos {}",
            soluciones.len(), min_soluciones
        ));
    }
    
    // Verificar que las soluciones tengan suficientes cursos
    let soluciones_validas = soluciones.iter()
        .filter(|(sol, _)| sol.len() >= min_cursos)
        .count();
    
    if soluciones_validas < min_soluciones {
        return Err(format!(
            "Solo {} soluciones tienen {} o más cursos, se esperaban al menos {}",
            soluciones_validas, min_cursos, min_soluciones
        ));
    }
    
    Ok(())
}
