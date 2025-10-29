// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica


use std::error::Error;
use std::collections::HashMap;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use crate::algorithms::pert::PertNode;
use crate::models::{Seccion, RamoDisponible};



/// Ejecutar la ruta crítica usando parámetros provistos por el usuario.
///
/// Esta versión acepta un `InputParams` (por ejemplo parseado desde JSON)
/// y devuelve las soluciones producidas por el planner de clique, lo que
/// facilita exponer el resultado vía HTTP o tests.
pub fn ejecutar_ruta_critica_with_params(
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    // Obtener ramos y secciones, delegar en la versión que acepta datos precomputados.
    let (ramos_disponibles, nombre_malla, _malla_leida) = crate::algorithms::get_ramo_critico();
    let (lista_secciones, mut ramos_actualizados, _oferta_leida) =
        crate::algorithms::extract_data(&ramos_disponibles, &nombre_malla);

    ejecutar_ruta_critica_with_precomputed(lista_secciones, ramos_actualizados, params)
}

/// Ejecutar la ruta crítica cuando ya se tienen `lista_secciones` y `ramos_actualizados`.
/// Esta variante evita volver a leer/extract_data y permite que `mod.rs` haga
/// la preparación (llamadas a `extract`) y luego invoque aquí la ejecución
/// final (lectura de porcentajes + planner que respeta preferencias).
pub fn ejecutar_ruta_critica_with_precomputed(
    lista_secciones: Vec<Seccion>,
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
    let porcentajes_path = "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx";
    if let Ok(pmap) = crate::excel::leer_porcentajes_aprobados(porcentajes_path) {
        // actualizar ramos_actualizados con la dificultad leída
        for (codigo, (porc, _total)) in pmap.into_iter() {
            if let Some(ramo) = ramos_actualizados.get_mut(&codigo) {
                ramo.dificultad = Some(porc);
            }
        }
    }

    // Construir un grafo PERT simple a partir de los ramos disponibles.
    // Uso heurístico: si `codigo_ref` existe lo usamos como prerequisito; else usamos
    // `numb_correlativo` para inferir precedencias adyacentes.
    let mut pert_graph: DiGraph<PertNode, ()> = DiGraph::new();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();

    for (codigo, ramo) in ramos_actualizados.iter() {
        let node = PertNode {
            codigo: codigo.clone(),
            nombre: ramo.nombre.clone(),
            es: None,
            ef: None,
            ls: None,
            lf: None,
            h: None,
        };
        let idx = pert_graph.add_node(node);
        node_map.insert(codigo.clone(), idx);
    }

    // Añadir aristas por `codigo_ref` donde exista
    for (codigo, ramo) in ramos_actualizados.iter() {
        if let Some(ref_code) = &ramo.codigo_ref {
            if ref_code != codigo {
                if let (Some(&from), Some(&to)) = (node_map.get(ref_code), node_map.get(codigo)) {
                    // from -> to (prerequisito)
                    let _ = pert_graph.add_edge(from, to, ());
                }
            }
        }
    }

    // Heurística por numero correlativo: unir i -> j si j = i+1
    for (a_code, a) in ramos_actualizados.iter() {
        for (b_code, b) in ramos_actualizados.iter() {
            if b.numb_correlativo == a.numb_correlativo + 1 {
                if let (Some(&from), Some(&to)) = (node_map.get(a_code), node_map.get(b_code)) {
                    // evitar duplicados
                    if pert_graph.find_edge(from, to).is_none() {
                        let _ = pert_graph.add_edge(from, to, ());
                    }
                }
            }
        }
    }

    // Ejecutar cálculo PERT para cada nodo (simplificado)
    for node_idx in pert_graph.node_indices() {
        // len_dag aproximado: número de nodos
        let len_dag = pert_graph.node_count() as i32;
        crate::algorithms::pert::set_values_recursive(&mut pert_graph, node_idx, len_dag);
    }

    // Propagar resultado PERT a ramos_actualizados (marcar críticos con holgura == 0)
    for (codigo, idx) in node_map.iter() {
        if let Some(pn) = pert_graph.node_weight(*idx) {
            if let Some(h) = pn.h {
                if let Some(r) = ramos_actualizados.get_mut(codigo) {
                    // Si la holgura es 0, reforzamos la bandera `critico`.
                    if h == 0 {
                        r.critico = true;
                    }
                }
            }
        }
    }

    // Llamar al planner que respeta preferencias de usuario (wrapper en algorithms)
    let soluciones = crate::algorithms::get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params);

    Ok(soluciones)
}

/// Compat: función simple que inicia la ruta crítica en modo no-parametrizado.
/// Para el uso HTTP/producción preferimos `ejecutar_ruta_critica_with_params`.
// Removed the compat no-op `ejecutar_ruta_critica` to avoid name clashes.

/// Versión sin parámetros para compatibilidad: llama a la versión con params
/// usando un `InputParams` vacío.
pub fn run_ruta_critica_solutions() -> Result<Vec<(Vec<(crate::models::Seccion, i32)>, i64)>, Box<dyn std::error::Error>> {
    println!("[rutacritica] Iniciando run_ruta_critica_solutions...");

    let (ramos_disponibles, nombre_excel_malla, _malla_leida) = crate::algorithms::get_ramo_critico();

    // Usar la función de extracción del submódulo de algorithms (ya incorpora la lógica detallada)
    let (lista_secciones, _ramos_actualizados) =
        crate::algorithms::extract::extract_data(ramos_disponibles.clone(), &nombre_excel_malla)?;
    let soluciones = crate::algorithms::get_clique_max_pond(&lista_secciones, &ramos_disponibles);

    println!("[rutacritica] Soluciones obtenidas: {}", soluciones.len());
    Ok(soluciones)
}
