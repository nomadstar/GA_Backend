/// Módulo de control de versiones: decide qué algoritmo usar
/// Permite cambiar entre versión lenta (original) y rápida (optimizada)

use std::collections::HashMap;
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::models::{Seccion, RamoDisponible};

/// Flag global para activar/desactivar versión optimizada
/// Por defecto: true (usar optimizado)
/// Para debugging/comparación: false (usar versión original)
static USE_OPTIMIZED: AtomicBool = AtomicBool::new(true);

/// Establecer si usar versión optimizada
pub fn set_use_optimized(use_opt: bool) {
    USE_OPTIMIZED.store(use_opt, Ordering::Relaxed);
}

/// Obtener estado actual
pub fn is_using_optimized() -> bool {
    USE_OPTIMIZED.load(Ordering::Relaxed)
}

/// Wrapper que elige automáticamente entre versión vieja y optimizada
pub fn extract_data(
    ramos_disponibles: HashMap<String, RamoDisponible>,
    nombre_excel_malla: &str,
    sheet: Option<&str>,
) -> Result<(Vec<Seccion>, HashMap<String, RamoDisponible>), Box<dyn Error>> {
    if is_using_optimized() {
        eprintln!("📊 Usando versión OPTIMIZADA (O(n) - rápida)");
        crate::algorithm::extract_optimizado::extract_data_optimizado(
            ramos_disponibles,
            nombre_excel_malla,
            sheet,
        )
    } else {
        eprintln!("📊 Usando versión ORIGINAL (O(n²) - lenta, solo para debug)");
        crate::algorithm::extract::extract_data(ramos_disponibles, nombre_excel_malla, sheet)
    }
}

/// Benchmark: comparar ambas versiones
#[cfg(test)]
pub fn benchmark_versions() {
    use std::time::Instant;

    eprintln!("\n🏁 BENCHMARK: Comparando versiones...\n");

    let malla = "MiMalla.xlsx";

    // Versión antigua
    eprintln!("\n📊 Versión ANTIGUA (O(n²)):");
    let initial_map_old = HashMap::new();
    let t0 = Instant::now();
    let result_old = crate::algorithm::extract::extract_data(
        initial_map_old,
        malla,
        None,
    );
    let time_old = t0.elapsed();
    match &result_old {
        Ok((sec, ramos)) => {
            eprintln!(
                "  ✅ Completado en {:?}: {} secciones, {} ramos",
                time_old,
                sec.len(),
                ramos.len()
            );
        }
        Err(e) => eprintln!("  ❌ Error: {}", e),
    }

    // Versión optimizada
    eprintln!("\n📊 Versión OPTIMIZADA (O(n)):");
    let initial_map_opt = HashMap::new();
    let t0 = Instant::now();
    let result_opt = crate::algorithm::extract_optimizado::extract_data_optimizado(
        initial_map_opt,
        malla,
        None,
    );
    let time_opt = t0.elapsed();
    match &result_opt {
        Ok((sec, ramos)) => {
            eprintln!(
                "  ✅ Completado en {:?}: {} secciones, {} ramos",
                time_opt,
                sec.len(),
                ramos.len()
            );
        }
        Err(e) => eprintln!("  ❌ Error: {}", e),
    }

    // Resumen
    if let (Ok((sec1, _)), Ok((sec2, _))) = (&result_old, &result_opt) {
        if sec1.len() == sec2.len() {
            eprintln!("\n✅ RESULTADOS IDÉNTICOS: Ambas versiones dan {} secciones", sec1.len());
        } else {
            eprintln!(
                "\n⚠️  RESULTADOS DIFERENTES: {} vs {}",
                sec1.len(),
                sec2.len()
            );
        }
        
        if time_opt.as_secs_f64() > 0.0 {
            let speedup = time_old.as_secs_f64() / time_opt.as_secs_f64();
            eprintln!("\n📈 SPEEDUP: {:.1}x más rápido", speedup);
        }
    }
}

 