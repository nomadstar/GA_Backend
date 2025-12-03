// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica

// Orquestador de la "Ruta Crítica" que integra módulos del crate `algorithm`:
// - lee malla/oferta/porcentajes (módulo excel)
// - ejecuta PERT (algorithm::pert) para marcar ramos críticos
// - ejecuta el planner (algorithm::clique) respetando filtros y restricciones
//
// Esta unidad debe ser el único punto que combine PERT + Planner; cualquier
// lógica de cálculo de grafos / heurísticas permanece en sus módulos.
use std::error::Error;
use crate::api_json::InputParams;
use crate::models::{Seccion, RamoDisponible};
use std::collections::HashMap;

pub fn ejecutar_ruta_critica_with_params(
    params: InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    eprintln!("🔁 [ruta::ejecutar_ruta_critica_with_params] iniciando pipeline...");

    // 1) Resolver paths de datafiles
    let (malla_pathbuf, oferta_pathbuf, porcentajes_pathbuf) = 
        crate::excel::resolve_datafile_paths(&params.malla)?;

    let malla_str = malla_pathbuf.to_string_lossy().to_string();
    let oferta_str = oferta_pathbuf.to_string_lossy().to_string();
    let porcentajes_str = porcentajes_pathbuf.to_string_lossy().to_string();

    eprintln!("   malla_path = {}", malla_str);
    eprintln!("   oferta_path = {}", oferta_str);
    eprintln!("   porcentajes_path = {}", porcentajes_str);

    // 2) Leer la malla + porcentajes -> HashMap<String, RamoDisponible>
    eprintln!("📥 Leyendo malla y porcentajes (optimizado)...");
    let mut ramos_map: HashMap<String, RamoDisponible> = 
        crate::excel::malla_optimizado::leer_malla_con_porcentajes_optimizado(&malla_str, &porcentajes_str)?;
    eprintln!("   ramos cargados: {}", ramos_map.len());

    // 3) Leer la oferta -> Vec<Seccion>
    eprintln!("📥 Leyendo oferta académica...");
    let lista_secciones: Vec<Seccion> = 
        crate::excel::leer_oferta_academica_excel(&oferta_str)?;
    eprintln!("   secciones cargadas: {}", lista_secciones.len());

    // 4) Ejecutar PERT sobre los ramos para marcar críticos y ajustar holguras
    eprintln!("🧭 Ejecutando PERT...");
    if let Err(e) = crate::algorithm::pert::build_and_run_pert(&mut ramos_map, &lista_secciones, &malla_str) {
        eprintln!("⚠️  PERT retornó aviso: {:?}", e);
    } else {
        eprintln!("   PERT completado: ramos actualizados (critico/holgura)");
    }

    // 5) Llamar al planner (clique) que respeta filtros/semestres/ventanas/profesores
    eprintln!("🧠 Ejecutando planner (clique) con filtros...");
    let soluciones = crate::algorithm::clique::get_clique_max_pond_with_prefs(&lista_secciones, &ramos_map, &params);

    eprintln!("✅ Pipeline completado: soluciones generadas = {}", soluciones.len());
    Ok(soluciones)
}

/// Función alternativa (compatibilidad): intenta cargar con malla por defecto
pub fn run_ruta_critica_solutions() -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    let params = InputParams {
        email: "default@example.com".to_string(),
        ramos_pasados: Vec::new(),
        ramos_prioritarios: Vec::new(),
        horarios_preferidos: Vec::new(),
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: None,
        ranking: None,
        filtros: None,
    };
    ejecutar_ruta_critica_with_params(params)
}

