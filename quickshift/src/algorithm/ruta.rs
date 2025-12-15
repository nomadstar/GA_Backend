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
// Nuevo import para comprobar solapamiento contra bloques prohibidos
use crate::algorithm::filters::solapan_horarios;
use std::collections::{HashMap, HashSet};

pub fn ejecutar_ruta_critica_with_params(
    mut params: InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    eprintln!("🔁 [ruta::ejecutar_ruta_critica_with_params] iniciando pipeline de 4 fases...");

    // =========================================================================
    // PHASE 0: Mapear códigos de ramos aprobados usando equivalencias
    // =========================================================================
    // Cargar equivalencias y mapear ramos_pasados
    let (malla_pathbuf, oferta_pathbuf, porcentajes_pathbuf) = 
        crate::excel::resolve_datafile_paths(&params.malla)?;
    let malla_str = malla_pathbuf.to_string_lossy().to_string();
    
    match crate::excel::cargar_equivalencias(&malla_str) {
        Ok(equivalencias) => {
            if !equivalencias.is_empty() {
                eprintln!("📋 PHASE 0: Mapeando ramos aprobados con equivalencias");
                eprintln!("   ✓ {} equivalencias cargadas", equivalencias.len());
                params.ramos_pasados = crate::excel::aplicar_equivalencias(&params.ramos_pasados, &equivalencias);
                eprintln!("   ✓ Ramos pasados mapeados a códigos de malla actual");
            }
        }
        Err(e) => {
            eprintln!("   ⚠️  No se pudieron cargar equivalencias: {}", e);
        }
    }

    // =========================================================================
    // PHASE 1: getRamoCritico + PERT
    // =========================================================================
    eprintln!("📋 PHASE 1: getRamoCritico + PERT");
    
    // 1a) Resolver paths de datafiles (ya hecho arriba, reutilizar)
    let oferta_str = oferta_pathbuf.to_string_lossy().to_string();
    let porcentajes_str = porcentajes_pathbuf.to_string_lossy().to_string();

    eprintln!("   malla_path = {}", malla_str);
    eprintln!("   oferta_path = {}", oferta_str);
    eprintln!("   porcentajes_path = {}", porcentajes_str);
    
    // 1b) Leer malla + porcentajes -> HashMap<String, RamoDisponible>
    eprintln!("   📥 Leyendo malla y porcentajes...");
    let mut ramos_disponibles: HashMap<String, RamoDisponible> = 
        if malla_str.to_uppercase().contains("MC") {
            // Usar parser especial para MC (Malla Curricular)
            eprintln!("   🔍 Detectado MC - usando parser especial");
            crate::excel::leer_mc_con_porcentajes_optimizado(&malla_str, &porcentajes_str)?
        } else {
            // Usar parser estándar para Malla2020 / MiMalla
            crate::excel::malla_optimizado::leer_malla_con_porcentajes_optimizado(&malla_str, &porcentajes_str)?
        };
    eprintln!("   ✓ ramos cargados: {}", ramos_disponibles.len());
    
    // 1c) PODADO DETERMINISTA: Filtrar ramos cuyo satisfacción de prerequisitos es imposible
    // REGLA DURA: Un ramo solo es viable si TODOS sus prerequisites están en ramos_pasados
    eprintln!("   🔪 PODADO: Filtrando ramos inviables (prerequisitos no satisfacibles)");
    let ramos_viable_map = crate::algorithm::pert::build_viable_ramos(&ramos_disponibles, &params.ramos_pasados);
    ramos_disponibles = ramos_viable_map.into_iter().collect();
    
    // =========================================================================
    // PHASE 2: extract_viable_sections
    // =========================================================================
    eprintln!("📋 PHASE 2: extract_viable_sections");
    // DEBUG: mostrar filtros y franjas recibidas para diagnóstico
    eprintln!("   [DEBUG] params.filtros={:?}", params.filtros);
    eprintln!("   [DEBUG] params.horarios_prohibidos={:?}", params.horarios_prohibidos);
    
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
    // NOTA: La validación de requisitos previos se maneja en clique.rs través del cálculo de max_sem
    // PERO: La LEY FUNDAMENTAL se garantiza porque la universidad no diseña
    //       ramos incompatibles en el mismo semestre
    eprintln!("   🔍 Filtrando secciones viables...");
    let passed_set: HashSet<String> = params.ramos_pasados
        .iter()
        .map(|s| s.to_uppercase())
        .collect();
    
    let lista_secciones_viables: Vec<Seccion> = lista_secciones
        .iter()
        .filter(|sec| {
            let sec_codigo_upper = sec.codigo.to_uppercase();

            if passed_set.contains(&sec_codigo_upper) {
                eprintln!("   ⊘ Excluyendo {} (ya aprobado)", sec.codigo);
                return false;
            }

            // Excluir si solapa con cualquier bloque prohibido pasado por el usuario
            if !params.horarios_prohibidos.is_empty() {
                eprintln!("   [DEBUG] Comprobando solapamiento contra franjas_prohibidas: {:?}", params.horarios_prohibidos);
                // sec.horario es Vec<String>
                if solapan_horarios(&sec.horario, &params.horarios_prohibidos) {
                    eprintln!("   ⊘ Excluyendo {} (solapa con franja prohibida)", sec.codigo);
                    return false;
                }
            }

            // Si existen filtros adicionales, aplicarlos aquí (ej: dias_horarios_libres estrictos)
            if let Some(ref filtros) = params.filtros {
                if let Some(ref dhl) = filtros.dias_horarios_libres {
                    if let Some(ref dias) = dhl.dias_libres_preferidos {
                        for dia_str in dias.iter() {
                            let dia_code = dia_str.to_uppercase();
                            for h in &sec.horario {
                                let segs = crate::algorithm::filters::expand_horario_entry(h); // reusar parser público
                                for (d, _s, _e) in segs.iter() {
                                    if &dia_code == d {
                                        eprintln!("   ⊘ Excluyendo {} (tiene clase en día que debe ser libre {})", sec.codigo, dia_code);
                                        return false;
                                    }
                                }
                            }
                        }
                    }
                }
            }

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
    
    // 3) Ejecutar búsqueda de cliques con preferencias del usuario
    let soluciones = crate::algorithm::clique::get_clique_max_pond_with_prefs(
        &lista_secciones_viables,
        &ramos_disponibles,
        &params,
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
    // PHASE 4: apply_filters (DEPRECADO - Los filtros se aplican en el clique)
    // =========================================================================
    eprintln!("📋 PHASE 4: apply_filters (skipped - filters applied in clique)");
    
    // Guardar una solución de backup para LEY FUNDAMENTAL ANTES de mover soluciones
    let mejor_solucion_backup = if soluciones_count > 0 { soluciones.get(0).cloned() } else { None };

    // Verificar si hay filtros activos (para validaciones posteriores)
    let has_active_filters = params.filtros
        .as_ref()
        .map(|f| {
            (f.dias_horarios_libres.as_ref().map(|d| d.habilitado).unwrap_or(false)) ||
            (f.ventana_entre_actividades.as_ref().map(|v| v.habilitado).unwrap_or(false)) ||
            (f.preferencias_profesores.as_ref().map(|p| p.habilitado).unwrap_or(false)) ||
            (f.balance_lineas.as_ref().map(|b| b.habilitado).unwrap_or(false))
        })
        .unwrap_or(false);
    
    // Aplicar FILTRADO ESTRICTO: eliminar soluciones que violen franjas prohibidas
    use crate::algorithm::filters::{apply_all_filters, solapan_horarios};

    // Función auxiliar: verifica si una solución contiene alguna sección que solape con
    // cualquiera de las franjas_prohibidas representadas como strings en params.horarios_prohibidos
    let solution_violates_prohibidos = |sol: &Vec<(Seccion, i32)>| -> bool {
        if params.horarios_prohibidos.is_empty() {
            return false;
        }
        for (s, _) in sol.iter() {
            if solapan_horarios(&s.horario, &params.horarios_prohibidos) {
                return true;
            }
        }
        false
    };

    // Primero, eliminar soluciones que violen directamente las cadenas de franjas prohibidas
    let mut soluciones_filtradas: Vec<(Vec<(Seccion, i32)>, i64)> = soluciones
        .into_iter()
        .filter(|(sol, _)| !solution_violates_prohibidos(sol))
        .collect();

    // Luego, si hay filtros estructurados en params.filtros, aplicarlos estrictamente
    if params.filtros.is_some() {
        soluciones_filtradas = apply_all_filters(soluciones_filtradas, &params.filtros);
    }

    // Ahora, seleccionar soluciones intentando maximizar cantidad de ramos,
    // pero siendo permisivos si no alcanzamos 10 resultados: intentar k=6..1
    let mut seleccionadas: Vec<(Vec<(Seccion, i32)>, i64)> = Vec::new();

    // Agrupar por longitud y recorrer desde 6 descendente hasta 1
    for k in (1..=6).rev() {
        // tomar las soluciones de longitud k, ordenar por score desc
        let mut grupo: Vec<_> = soluciones_filtradas
            .iter()
            .filter(|(sol, _)| sol.len() == k)
            .cloned()
            .collect();
        grupo.sort_by(|a, b| b.1.cmp(&a.1));

        for item in grupo.into_iter() {
            if seleccionadas.len() >= 10 { break; }
            seleccionadas.push(item);
        }

        if seleccionadas.len() >= 10 { break; }
    }

    // Si no se seleccionó nada (caso extremo), mantener las mejores hasta 10
    if seleccionadas.is_empty() {
        eprintln!("   ⚠️  No se encontraron soluciones por longitud; devolviendo las mejores disponibles");
        seleccionadas = soluciones_filtradas.into_iter().take(20).collect();
    }

    let soluciones_filtradas_count = seleccionadas.len();
    eprintln!("   ✓ soluciones que cumplen filtros (seleccionadas): {}", soluciones_filtradas_count);

    let mut resultado: Vec<_> = seleccionadas.into_iter().take(20).collect();
    
    // =====================================================================
    // VALIDACIÓN CRÍTICA - LEY FUNDAMENTAL
    // =====================================================================
    // LEY: Si no hay filtros activos Y quedan cursos por aprobar,
    // SIEMPRE debe haber al menos 1 solución
    
    let cursos_por_aprobar = lista_secciones_viables.len();
    
    if resultado.is_empty() && !has_active_filters && cursos_por_aprobar > 0 {
        // FALLBACK: LEY FUNDAMENTAL - Si no hay filtros y hay cursos disponibles,
        // MUST retornar al menos 1 solución
        eprintln!("❌ LEY FUNDAMENTAL VIOLADA: Intentando recuperación...");
        eprintln!("   - Soluciones en PHASE 3: {}", soluciones_count);
        eprintln!("   - Soluciones después PHASE 4: {}", soluciones_filtradas_count);
        
        if let Some(sol) = mejor_solucion_backup {
            // Hay soluciones de PHASE 3 pero fueron filtradas por PHASE 4
            // Retornar la mejor solución sin filtros
            eprintln!("   [FALLBACK] Retornando mejor solución sin aplicar filtros PHASE 4...");
            resultado.push(sol);
        } else {
            // No hay soluciones ni siquiera en PHASE 3
            eprintln!("❌ ✋ LEY FUNDAMENTAL VIOLADA COMPLETAMENTE ✋ ❌");
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
        }
    } else if resultado.is_empty() && has_active_filters && cursos_por_aprobar > 0 {
        // FALLBACK PARA FILTROS ACTIVOS: Si hay filtros muy restrictivos que eliminan TODO,
        // retornar al menos 1 solución (el mejor curso disponible)
        eprintln!("⚠️  AVISO (FALLBACK): Filtros muy restrictivos eliminaron todas las soluciones");
        eprintln!("   - Soluciones en PHASE 3: {}", soluciones_count);
        eprintln!("   - Soluciones después PHASE 4: {}", soluciones_filtradas_count);
        
        if let Some(sol) = mejor_solucion_backup {
            eprintln!("   [FALLBACK] Retornando mejor solución incluso sin cumplir todos los filtros...");
            resultado.push(sol);
        }
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
        horarios_prohibidos: Vec::new(),
        malla: "MiMalla.xlsx".to_string(),
        anio: None,
        sheet: None,
        student_ranking: None,
        ranking: None,
        filtros: None,
        optimizations: Vec::new(),
    };
    ejecutar_ruta_critica_with_params(params)
}