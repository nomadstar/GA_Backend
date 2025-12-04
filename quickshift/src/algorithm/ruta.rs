// ruta.rs - Orquestador que implementa el pipeline de 4 fases del RutaCritica.py
//
// Pipeline correcto (basado en RutaCritica.py de Python):
// PHASE 1: getRamoCritico + build_and_run_pert
//   - Cargar malla + porcentajes
//   - Construir grafo PERT con prerequisites
//   - Calcular ES, EF, LS, LF, H, criticidad
//   - Output: ramos_disponibles con {critico, holgura, numb_correlativo}
//
// PHASE 2: extract_viable_sections
//   - Cargar oferta académica
//   - Filtrar secciones: electivos solo si TODOS sus prerequisites están aprobados
//   - Output: lista_secciones filtrada
//
// PHASE 3: clique_search (algorithm::clique::get_clique_max_pond_with_prefs)
//   - Calcular prioridades: CC+UU+KK+SS (8 dígitos)
//   - Encontrar máxima clique ponderada con max 6 ramos
//   - Iterar hasta 10 soluciones
//   - Output: Vec<(Vec<Seccion>, i64)> ordenado por score descendente
//
// PHASE 4: apply_filters
//   - (Actualmente delegado al frontend; aquí solo retornamos soluciones)
//   - Usuario puede filtrar por horarios_preferidos, profesores, etc.

use std::error::Error;
use crate::api_json::InputParams;
use crate::models::{Seccion, RamoDisponible};
use std::collections::{HashMap, HashSet};

pub fn ejecutar_ruta_critica_with_params(
    params: InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    eprintln!("🔁 [ruta::ejecutar_ruta_critica_with_params] iniciando pipeline de 4 fases...");

    // =========================================================================
    // PHASE 1: getRamoCritico + PERT
    // =========================================================================
    eprintln!("📋 PHASE 1: getRamoCritico + PERT");
    
    // 1a) Resolver paths de datafiles
    let (malla_pathbuf, oferta_pathbuf, porcentajes_pathbuf) = 
        crate::excel::resolve_datafile_paths(&params.malla)?;

    let malla_str = malla_pathbuf.to_string_lossy().to_string();
    let oferta_str = oferta_pathbuf.to_string_lossy().to_string();
    let porcentajes_str = porcentajes_pathbuf.to_string_lossy().to_string();

    eprintln!("   malla_path = {}", malla_str);
    eprintln!("   oferta_path = {}", oferta_str);
    eprintln!("   porcentajes_path = {}", porcentajes_str);
    
    // 1b) Leer malla + porcentajes -> HashMap<String, RamoDisponible>
    eprintln!("   📥 Leyendo malla y porcentajes...");
    let mut ramos_disponibles: HashMap<String, RamoDisponible> = 
        crate::excel::malla_optimizado::leer_malla_con_porcentajes_optimizado(&malla_str, &porcentajes_str)?;
    eprintln!("   ✓ ramos cargados: {}", ramos_disponibles.len());
    
    // =========================================================================
    // PHASE 2: extract_viable_sections
    // =========================================================================
    eprintln!("📋 PHASE 2: extract_viable_sections");
    
    // 2a) Leer oferta académica -> Vec<Seccion>
    eprintln!("   📥 Leyendo oferta académica...");
    let lista_secciones: Vec<Seccion> = 
        crate::excel::leer_oferta_academica_excel(&oferta_str)?;
    eprintln!("   ✓ secciones cargadas: {}", lista_secciones.len());
    
    // 2b) Ejecutar PERT ANTES de filtrar secciones
    // (porque necesitamos critico/holgura/numb_correlativo propagados)
    eprintln!("   🧭 Ejecutando PERT (primera pasada)...");
    if let Err(e) = crate::algorithm::pert::build_and_run_pert(
        &mut ramos_disponibles, 
        &lista_secciones, 
        &malla_str
    ) {
        eprintln!("   ⚠️  PERT aviso: {:?}", e);
    } else {
        eprintln!("   ✓ PERT completado: ramos actualizados (critico/holgura)");
    }
    
    // 2c) Filtrar secciones viables según reglas Python:
    // - Excluir ramos ya aprobados (ramos_pasados)
    // - Excluir ramos cuyos prerequisitos NO estén en ramos_pasados
    eprintln!("   🔍 Filtrando secciones viables...");
    let passed_set: HashSet<String> = params.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    // Crear un mapa de código -> RamoDisponible para búsquedas rápidas
    let codigo_to_ramo: HashMap<String, &RamoDisponible> = ramos_disponibles.iter()
        .map(|(k, v)| (k.to_uppercase(), v))
        .collect();
    
    let lista_secciones_viables: Vec<Seccion> = lista_secciones
        .iter()
        .filter(|sec| {
            let sec_codigo_upper = sec.codigo.to_uppercase();
            
            // Excluir si ya fue aprobado
            if passed_set.contains(&sec_codigo_upper) {
                eprintln!("   ⊘ Excluyendo {} (ya aprobado)", sec.codigo);
                return false;
            }
            
            // Obtener el ramo de la malla
            if let Some(ramo) = codigo_to_ramo.get(&sec_codigo_upper) {
                // Verificar si TODOS los prerequisitos están en ramos_pasados
                // Un ramo es viable si:
                // 1. No tiene prerequisito (codigo_ref == id), O
                // 2. Su prerequisito está en ramos_pasados
                
                if let Some(prereq_id) = ramo.codigo_ref {
                    if prereq_id != ramo.id {
                        // Tiene prerequisito, buscar ese ramo
                        if let Some(prereq_ramo) = ramos_disponibles.values().find(|r| r.id == prereq_id) {
                            // El prerequisito debe estar en ramos_pasados
                            if !passed_set.contains(&prereq_ramo.codigo.to_uppercase()) {
                                eprintln!("   ⊘ Excluyendo {} (prerequisito {} no aprobado)", 
                                         sec.codigo, prereq_ramo.codigo);
                                return false;
                            }
                        }
                    }
                }
                true
            } else {
                // Si no está en la malla, lo incluimos
                true
            }
        })
        .cloned()
        .collect();
    
    eprintln!("   ✓ secciones viables: {} (de {})", lista_secciones_viables.len(), 
              lista_secciones.len());
    
    // =========================================================================
    // PHASE 3: clique_search
    // =========================================================================
    eprintln!("📋 PHASE 3: clique_search");
    
    // 3) Ejecutar búsqueda de máxima clique ponderada
    // (implementada en algorithm::clique::get_clique_max_pond_with_prefs)
    let soluciones = crate::algorithm::clique::get_clique_max_pond_with_prefs(
        &lista_secciones_viables, 
        &ramos_disponibles, 
        &params
    );
    
    eprintln!("   ✓ clique search completado: {} soluciones antes de filtrar", soluciones.len());
    
    // =========================================================================
    // PHASE 4: apply_filters
    // =========================================================================
    eprintln!("📋 PHASE 4: apply_filters");
    
    let soluciones_filtradas = crate::algorithm::filters::apply_all_filters(
        soluciones, 
        &params.filtros
    );
    
    eprintln!("   ✓ soluciones después de filtrar: {}", soluciones_filtradas.len());
    
    // Retornar máximo 10 soluciones que hayan pasado los filtros
    let resultado: Vec<_> = soluciones_filtradas.into_iter().take(10).collect();
    
    eprintln!("✅ Pipeline completado: {} soluciones (máximo 10)", resultado.len());
    Ok(resultado)
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