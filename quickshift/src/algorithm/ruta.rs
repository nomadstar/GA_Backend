// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica

use std::collections::HashMap;
use std::error::Error;
use crate::models::{Seccion, RamoDisponible};
// ahora puedes llamar: extract::extract_data(...), clique::get_clique_with_user_prefs(...), conflict::horarios_tienen_conflicto(...), pert::set_values_recursive...

use super::{pert, clique, extract_controller};
// ahora puedes usar: pert::build_and_run_pert(...), extract::extract_data(...), clique::get_clique_max_pond(...), conflict::horarios_tienen_conflicto...



/// Ejecutar la ruta crítica usando parámetros provistos por el usuario.
///
/// Esta versión acepta un `InputParams` (por ejemplo parseado desde JSON)
/// y devuelve las soluciones producidas por el planner de clique, lo que
/// facilita exponer el resultado vía HTTP o tests.
pub fn ejecutar_ruta_critica_with_params(
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    // Obtener ramos y secciones, delegar en la versión que acepta datos precomputados.
    // Use the malla and optional sheet provided in params to extract data.
    let initial_map: std::collections::HashMap<String, RamoDisponible> = std::collections::HashMap::new();
    let sheet_opt = params.sheet.as_deref();
    let (lista_secciones, ramos_actualizados) = match extract_controller::extract_data(initial_map, &params.malla, sheet_opt) {
        Ok((ls, ra)) => (ls, ra),
        Err(e) => return Err(e),
    };

    ejecutar_ruta_critica_with_precomputed(lista_secciones, ramos_actualizados, params)
}

/// Ejecutar la ruta crítica cuando ya se tienen `lista_secciones` y `ramos_actualizados`.
/// Esta variante evita volver a leer/extract_data y permite que `mod.rs` haga
/// la preparación (llamadas a `extract`) y luego invoque aquí la ejecución
/// final (lectura de porcentajes + planner que respeta preferencias).
pub fn ejecutar_ruta_critica_with_precomputed(
    mut lista_secciones: Vec<Seccion>,
    mut ramos_actualizados: HashMap<String, RamoDisponible>,
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    println!("rutacritica::ruta -> ejecutar_ruta_critica_with_precomputed");

    // Validaciones mínimas
    if params.email.trim().is_empty() {
        return Err(Box::<dyn Error>::from("email is required in InputParams"));
    }

    // Intentar leer porcentajes de aprobados desde el archivo garantizado
    // y usarlo para poblar `RamoDisponible.dificultad`.
    let data_dir = crate::excel::get_datafiles_dir();
    let porcentajes_path = data_dir.join("PA2025-1.xlsx");
    if let Ok(pmap) = crate::excel::leer_porcentajes_aprobados(porcentajes_path.to_str().unwrap_or("")) {
        // actualizar ramos_actualizados con la dificultad leída
        for (codigo, (porc, _total)) in pmap.into_iter() {
            if let Some(ramo) = ramos_actualizados.get_mut(&codigo) {
                ramo.dificultad = Some(porc);
            }
        }
    }
    // Filter out empty/invalid ramos that may appear in some OA files (e.g. blank rows
    // in Oferta Academica). These ramos should not participate in the critical path
    // computation. We consider a ramo invalid if its `nombre` or `codigo` are empty
    // (after trimming). Remove them from `ramos_actualizados` and from `lista_secciones`.
    let mut invalid_codes: Vec<String> = Vec::new();
    for (code, ramo) in ramos_actualizados.iter() {
        if ramo.nombre.trim().is_empty() || ramo.codigo.trim().is_empty() {
            invalid_codes.push(code.clone());
        }
    }
    if !invalid_codes.is_empty() {
        eprintln!("INFO: excluding {} empty/invalid ramos from ruta critica: {:?}", invalid_codes.len(), invalid_codes);
        for c in invalid_codes.iter() {
            ramos_actualizados.remove(c);
        }
        // Remove matching sections from lista_secciones
        lista_secciones.retain(|s| !invalid_codes.iter().any(|ic| ic.eq_ignore_ascii_case(&s.codigo)));
    }

    // Delegar la construcción y ejecución del PERT al módulo `pert`.
    if let Err(e) = pert::build_and_run_pert(&mut ramos_actualizados, &lista_secciones, &params.malla) {
        return Err(e);
    }

    // Decidir cuál planner usar: si el usuario NO proporcionó preferencias
    // adicionales (solo entregó `ramos_pasados`) Y no habilitó filtros opcionales,
    // usamos la versión sin prefs `get_clique_max_pond`.
    // En caso contrario usamos la variante que respeta preferencias `get_clique_with_user_prefs`.
    // Reglas 3-6 (filtros opcionales) se aplican aquí si están habilitadas.
    let hay_filtros_habilitados = params.filtros.as_ref()
        .map(|f| {
            f.dias_horarios_libres.as_ref().map(|d| d.habilitado).unwrap_or(false)
                || f.ventana_entre_actividades.as_ref().map(|v| v.habilitado).unwrap_or(false)
                || f.preferencias_profesores.as_ref().map(|p| p.habilitado).unwrap_or(false)
                || f.balance_lineas.as_ref().map(|b| b.habilitado).unwrap_or(false)
        })
        .unwrap_or(false);

    let solo_pasados = params.ramos_prioritarios.is_empty()
        && params.horarios_preferidos.is_empty()
        && params.ranking.is_none()
        && params.student_ranking.is_none()
        && !hay_filtros_habilitados;

    let soluciones = if solo_pasados {
        clique::get_clique_max_pond(&lista_secciones, &ramos_actualizados)
    } else {
        clique::get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params)
    };

    Ok(soluciones)
}

