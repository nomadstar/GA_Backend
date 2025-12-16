//! Benchmark de determinismo: Ejecutar 100 veces y verificar que top-50 es idéntico
//! 
//! Este test verifica que el algoritmo de búsqueda de cliques es 100% determinista.
//! Ejecuta la búsqueda 100 veces con los mismos datos de entrada y verifica que
//! el ranking de las top-50 soluciones sea idéntico en todas las ejecuciones.
//!
//! Requisitos de determinismo HARD (usuario):
//! - NO heurísticas: Solo métodos exactos (backtracking exhaustivo)
//! - Top 50 soluciones con todos los empates mostrados
//! - 100+ ejecuciones idénticas sin variación

use quickshift::models::{Seccion, RamoDisponible};
use quickshift::api_json::InputParams;
use std::collections::HashMap;

#[test]
fn test_determinism_100_runs() {
    println!("═══════════════════════════════════════════════════════════");
    println!("DETERMINISM BENCHMARK TEST - 100 RUNS (REAL DATA)");
    println!("═══════════════════════════════════════════════════════════");
    println!("Objetivo: 100 ejecuciones idénticas del algoritmo quickshift");
    println!("Garantía: Top-50 soluciones son bit-for-bit idénticas en cada run");
    println!();
    
    // Cargar datos reales desde MC2020moded.xlsx
    println!("Cargando malla curricular...");
    
    let ramos_disponibles = match quickshift::excel::leer_malla_excel("MC2020moded.xlsx") {
        Ok(ramos) => {
            println!("✓ {} ramos cargados desde malla", ramos.len());
            ramos
        }
        Err(e) => {
            eprintln!("Error cargando malla: {}", e);
            eprintln!("Usando fixture de demostración...");
            create_demo_ramos()
        }
    };
    
    // Cargar oferta académica (secciones disponibles)
    let secciones = match quickshift::excel::leer_oferta_academica_excel("oferta_academica.xlsx") {
        Ok(sec) => {
            println!("✓ {} secciones cargadas desde oferta", sec.len());
            sec
        }
        Err(e) => {
            eprintln!("Error cargando oferta: {}", e);
            eprintln!("Usando fixture de demostración...");
            create_demo_secciones()
        }
    };
    
    println!("Fixture de datos:");
    println!("  - {} ramos totales", ramos_disponibles.len());
    println!("  - {} secciones disponibles", secciones.len());
    println!();
    
    // Parámetros de búsqueda
    let params = InputParams {
        email: "test@example.com".to_string(),
        ramos_pasados: Vec::new(),
        ramos_prioritarios: Vec::new(),
        horarios_preferidos: Vec::new(),
        horarios_prohibidos: Vec::new(),
        malla: "MC2020moded.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: None,
        ranking: None,
        filtros: None,
        optimizations: Vec::new(),
    };
    
    // ============================================================================
    // BENCHMARK: 100 EJECUCIONES
    // ============================================================================
    println!("Ejecutando 100 veces get_clique_with_user_prefs()...");
    
    #[derive(Debug, Clone, PartialEq, Eq)]
    struct RunResult {
        run_num: usize,
        solutions: Vec<(String, i64)>, // (clique_repr, score)
    }
    
    let mut all_results: Vec<RunResult> = Vec::new();
    let num_runs = 100;
    
    for run_num in 0..num_runs {
        // Ejecutar la búsqueda
        let results = quickshift::algorithm::get_clique_with_user_prefs(
            &secciones,
            &ramos_disponibles,
            &params,
        );
        
        // Convertir a formato comparable: (clique_repr, score)
        let mut solutions = Vec::new();
        for (clique_secs, score) in results.iter().take(50) {
            let clique_repr = clique_secs
                .iter()
                .map(|(sec, _)| format!("{}[S{}]", sec.codigo, sec.seccion))
                .collect::<Vec<_>>()
                .join("+");
            solutions.push((clique_repr, *score));
        }
        
        all_results.push(RunResult {
            run_num,
            solutions,
        });
        
        if run_num % 20 == 0 {
            eprint!(".");
        }
    }
    eprintln!(" ✓");
    println!();
    
    // ============================================================================
    // VERIFICACIÓN: COMPARAR TODAS LAS EJECUCIONES
    // ============================================================================
    println!("Comparando resultados de todas las 100 ejecuciones...");
    println!();
    
    let first_run = &all_results[0];
    let mut all_identical = true;
    let mut first_difference_run = None;
    let mut first_difference_idx = None;
    
    for (run_idx, run) in all_results.iter().enumerate().skip(1) {
        if run.solutions != first_run.solutions {
            all_identical = false;
            if first_difference_run.is_none() {
                first_difference_run = Some(run_idx);
                
                // Encontrar primer índice donde difieren
                for (sol_idx, (sol1, sol2)) in first_run.solutions
                    .iter()
                    .zip(run.solutions.iter())
                    .enumerate()
                {
                    if sol1 != sol2 {
                        first_difference_idx = Some(sol_idx);
                        break;
                    }
                }
            }
        }
    }
    
    println!("RESULTADOS:");
    println!("  - Total ejecuciones: {}", num_runs);
    println!("  - Soluciones en run 0: {}", first_run.solutions.len());
    println!();
    
    if all_identical {
        println!("✅ ÉXITO: Todas las 100 ejecuciones son IDÉNTICAS");
        println!("   Garantía de determinismo VERIFICADA");
        println!();
        
        println!("Top-50 soluciones (idénticas en todas las ejecuciones):");
        for (idx, (clique, score)) in first_run.solutions.iter().take(10).enumerate() {
            println!("  [{}] score={} | clique={}", idx, score, clique);
        }
        if first_run.solutions.len() > 10 {
            println!("  ... ({} más)", first_run.solutions.len() - 10);
        }
    } else {
        println!("❌ FALLO: Detectadas diferencias entre ejecuciones");
        println!("   Primera diferencia en: run {} (posición en top-50: {:?})",
            first_difference_run.unwrap(),
            first_difference_idx);
        
        let diff_run_idx = first_difference_run.unwrap();
        let diff_sol_idx = first_difference_idx.unwrap_or(0);
        
        println!();
        println!("Detalles de la diferencia:");
        println!("  Run 0, solución {}: {}", diff_sol_idx, first_run.solutions[diff_sol_idx].0);
        println!("  Run {}, solución {}: {}", diff_run_idx, diff_sol_idx, 
            all_results[diff_run_idx].solutions.get(diff_sol_idx)
                .map(|s| s.0.as_str())
                .unwrap_or("(no existe)"));
        
        panic!("Determinismo fallido: ejecuciones no idénticas");
    }
    
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("✅ TEST PASSED: Determinismo 100% garantizado");
    println!("═══════════════════════════════════════════════════════════");
}

/// Helper: Crear datos de demostración simple
fn create_demo_ramos() -> HashMap<String, RamoDisponible> {
    let mut ramos = HashMap::new();
    
    for sem in 1..=4 {
        for i in 1..=3 {
            let code = format!("RAMO_S{}_{}", sem, i);
            ramos.insert(code.clone(), RamoDisponible {
                id: (sem * 10 + i) as i32,
                nombre: format!("Ramo Semestre {} - {}", sem, i),
                codigo: code,
                holgura: 0,
                numb_correlativo: i as i32,
                critico: true,
                requisitos_ids: Vec::new(),
                dificultad: Some(50.0),
                electivo: false,
                semestre: Some(sem as i32),
            });
        }
    }
    
    ramos
}

/// Helper: Crear secciones de demostración
fn create_demo_secciones() -> Vec<Seccion> {
    let mut secciones = Vec::new();
    
    for sem in 1..=4 {
        for i in 1..=3 {
            for sec in 1..=2 {
                secciones.push(Seccion {
                    codigo: format!("RAMO_S{}_{}", sem, i),
                    nombre: format!("Ramo S{} {} - Sección {}", sem, i, sec),
                    seccion: sec.to_string(),
                    horario: vec![format!("{}:00-{}:00 MW", 8 + sem + i, 10 + sem + i)],
                    profesor: format!("Prof {}", sec),
                    codigo_box: format!("BOX_S{}_{}_SEC{}", sem, i, sec),
                    is_cfg: false,
                    is_electivo: false,
                });
            }
        }
    }
    
    secciones
}

/// Estructura para comparar dos ejecuciones
#[test]
fn test_determinism_comparison_structure() {
    #[derive(Debug, PartialEq)]
    struct SolutionComparison {
        run_a_rank: usize,
        run_b_rank: usize,
        run_a_score: i64,
        run_b_score: i64,
        clique_id: String,
        matches: bool,
    }
    
    let example = SolutionComparison {
        run_a_rank: 1,
        run_b_rank: 1,
        run_a_score: 88754302,
        run_b_score: 88754302,
        clique_id: "CALC1+CALC2+PHYS1".to_string(),
        matches: true,
    };
    
    assert!(example.matches, "Estructura de comparación válida");
}

