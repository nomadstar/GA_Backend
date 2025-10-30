// ruta.rs - orquestador que combina extracción y clique para producir la ruta crítica

use std::collections::HashMap;
use std::error::Error;
use petgraph::graph::{NodeIndex, DiGraph};

use crate::models::{Seccion, RamoDisponible, PertNode};
// ahora puedes llamar: extract::extract_data(...), clique::get_clique_with_user_prefs(...), conflict::horarios_tienen_conflicto(...), pert::set_values_recursive...



/// Ejecutar la ruta crítica usando parámetros provistos por el usuario.
///
/// Esta versión acepta un `InputParams` (por ejemplo parseado desde JSON)
/// y devuelve las soluciones producidas por el planner de clique, lo que
/// facilita exponer el resultado vía HTTP o tests.
pub fn ejecutar_ruta_critica_with_params(
    params: crate::api_json::InputParams,
) -> Result<Vec<(Vec<(Seccion, i32)>, i64)>, Box<dyn Error>> {
    // Obtener ramos y secciones, delegar en la versión que acepta datos precomputados.
    let (ramos_disponibles, nombre_malla, _malla_leida) = crate::algorithm::get_ramo_critico();
    let (lista_secciones, ramos_actualizados) = match crate::algorithm::extract_data(ramos_disponibles.clone(), &nombre_malla) {
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

    // Añadir aristas usando el mapa de prerequisitos leído desde la malla (hojas adicionales).
    let malla_name_for_prereq = params.malla.clone();
    if let Ok(pr_map) = crate::excel::leer_prerequisitos(&malla_name_for_prereq) {
        for (codigo, prereqs) in pr_map.into_iter() {
            for prereq in prereqs.into_iter() {
                if let (Some(&from), Some(&to)) = (node_map.get(&prereq), node_map.get(&codigo)) {
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
    crate::algorithm::pert::set_values_recursive(&mut pert_graph, node_idx, len_dag);
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

    // Decidir cuál planner usar: si el usuario NO proporcionó preferencias
    // adicionales (solo entregó `ramos_pasados`) usamos la versión sin prefs
    // `get_clique_max_pond`. En caso contrario usamos la variante que respeta
    // preferencias `get_clique_with_user_prefs`.
    let solo_pasados = params.ramos_prioritarios.is_empty()
        && params.horarios_preferidos.is_empty()
        && params.ranking.is_none()
        && params.student_ranking.is_none();

    let soluciones = if solo_pasados {
        crate::algorithm::get_clique_max_pond(&lista_secciones, &ramos_actualizados)
    } else {
        crate::algorithm::get_clique_with_user_prefs(&lista_secciones, &ramos_actualizados, &params)
    };

    Ok(soluciones)
}
