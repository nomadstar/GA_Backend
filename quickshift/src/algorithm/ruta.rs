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
    
    // 2c) Filtrar secciones viables:
    // RULE ESTRICTA: Excluir SOLO ramos ya aprobados (ramos_pasados)
    // NO excluimos por prerequisitos no satisfechos - dejar que el clique lo resuelva
    eprintln!("   🔍 Filtrando secciones viables...");
    let passed_set: HashSet<String> = params.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    let lista_secciones_viables: Vec<Seccion> = lista_secciones
        .iter()
        .filter(|sec| {
            let sec_codigo_upper = sec.codigo.to_uppercase();
            
            // Excluir si ya fue aprobado - ESTO ES OBLIGATORIO
            if passed_set.contains(&sec_codigo_upper) {
                eprintln!("   ⊘ Excluyendo {} (ya aprobado)", sec.codigo);
                return false;
            }
            
            // Incluir todo lo demás - el clique seleccionará lo mejor
            true
        })
        .cloned()
        .collect();
    
    eprintln!("   ✓ secciones viables: {} (de {})", lista_secciones_viables.len(), 
              lista_secciones.len());
    
    // =========================================================================
    // PHASE 3: clique_search
    // =========================================================================
    eprintln!("📋 PHASE 3: clique_search");
    
    // VALIDACIÓN: Debe haber al menos algunas secciones viables
    if lista_secciones_viables.is_empty() {
        eprintln!("❌ ERROR: No hay secciones viables después de filtrar");
        eprintln!("   Posibles causas:");
        eprintln!("   - Todos los cursos están en ramos_pasados");
        eprintln!("   - El archivo de oferta académica está vacío");
        eprintln!("   - Hay un problema en PHASE 2");
        return Ok(Vec::new());
    }
    
    // 3) Ejecutar búsqueda de máxima clique ponderada
    // (implementada en algorithm::clique::get_clique_max_pond_with_prefs)
    let soluciones = crate::algorithm::clique::get_clique_max_pond_with_prefs(
        &lista_secciones_viables, 
        &ramos_disponibles, 
        &params
    );
    
    // Log del resultado del clique y guardar el count
    let soluciones_count = soluciones.len();
    eprintln!("   ✓ clique search completado: {} soluciones antes de filtrar", soluciones_count);
    
    // VALIDACIÓN: El clique debe generar al menos 1 solución si hay secciones viables
    if soluciones.is_empty() && !lista_secciones_viables.is_empty() {
        eprintln!("⚠️  AVISO: El clique no generó soluciones a pesar de tener {} secciones viables", 
                  lista_secciones_viables.len());
        eprintln!("   Esto puede indicar que los cursos viables son incompatibles entre sí");
    }
    
    // =========================================================================
    // PHASE 4: apply_filters
    // =========================================================================
    eprintln!("📋 PHASE 4: apply_filters");
    
    // Verificar si hay filtros activos
    let has_active_filters = params.filtros
        .as_ref()
        .map(|f| {
            (f.dias_horarios_libres.as_ref().map(|d| d.habilitado).unwrap_or(false)) ||
            (f.ventana_entre_actividades.as_ref().map(|v| v.habilitado).unwrap_or(false)) ||
            (f.preferencias_profesores.as_ref().map(|p| p.habilitado).unwrap_or(false)) ||
            (f.balance_lineas.as_ref().map(|b| b.habilitado).unwrap_or(false))
        })
        .unwrap_or(false);
    
    let soluciones_filtradas = crate::algorithm::filters::apply_all_filters(
        soluciones, 
        &params.filtros
    );
    
    let soluciones_filtradas_count = soluciones_filtradas.len();
    eprintln!("   ✓ soluciones después de filtrar: {}", soluciones_filtradas_count);
    
    // Retornar máximo 10 soluciones que hayan pasado los filtros
    let resultado: Vec<_> = soluciones_filtradas.into_iter().take(10).collect();
    
    // =====================================================================
    // VALIDACIÓN CRÍTICA - LEY FUNDAMENTAL
    // =====================================================================
    // LEY: Si no hay filtros activos Y quedan cursos por aprobar,
    // SIEMPRE debe haber al menos 1 solución
    
    let cursos_por_aprobar = lista_secciones_viables.len();
    
    if resultado.is_empty() && !has_active_filters && cursos_por_aprobar > 0 {
        eprintln!("❌ ✋ LEY VIOLADA ✋ ❌");
        eprintln!("   VIOLACIÓN: No hay soluciones pero:");
        eprintln!("   - Hay {} cursos disponibles para aprobar", cursos_por_aprobar);
        eprintln!("   - NO hay filtros activos");
        eprintln!("   - Esto es IMPOSIBLE y indica un BUG EN EL SISTEMA");
        eprintln!();
        eprintln!("   Diagnóstico:");
        eprintln!("   - Soluciones generadas en PHASE 3: {}", soluciones_count);
        eprintln!("   - Soluciones que pasaron filtros: {}", soluciones_filtradas_count);
        eprintln!("   - Estado del clique: FALLO CRÍTICO");
        eprintln!();
        eprintln!("   Acción: Este error debe ser investigado inmediatamente");
        // Retornamos vacío pero con log evidente
    }
    
    if resultado.is_empty() && has_active_filters && cursos_por_aprobar > 0 {
        eprintln!("⚠️  AVISO: No hay soluciones que pasen los filtros aplicados");
        eprintln!("   - Cursos disponibles: {}", cursos_por_aprobar);
        eprintln!("   - Considere relajar algunos filtros para obtener resultados");
    }
    
    if resultado.is_empty() && cursos_por_aprobar == 0 {
        eprintln!("✅ INFORMACIÓN: Todos los cursos han sido aprobados");
        eprintln!("   - Felicidades, has completado el programa");
    }
    
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