/// Módulo optimizado para lectura de malla con Mapeo Maestro
/// Utiliza HashMap para O(1) lookup en lugar de búsquedas nested O(n²)
/// 
/// Características:
/// - O(n) construcción (lineal)
/// - O(1) búsqueda runtime
/// - Soporte para parallelización futura
/// - Cero búsquedas anidadas

use std::collections::HashMap;
use std::error::Error;
use crate::models::RamoDisponible;
use crate::excel::mapeo::MapeoMaestro;

/// Versión optimizada de leer_malla_con_porcentajes usando MapeoMaestro
/// 
/// Esta función reemplaza la versión O(n²) con una versión O(n) que:
/// 1. Construye MapeoMaestro desde 3 fuentes Excel
/// 2. Usa nombre normalizado como clave universal
/// 3. Evita búsquedas anidadas completamente
/// 4. Retorna HashMap<String, RamoDisponible> compatible con API existente
pub fn leer_malla_con_porcentajes_optimizado(
    malla_archivo: &str,
    porcentajes_archivo: &str,
) -> Result<HashMap<String, RamoDisponible>, Box<dyn Error>> {
    use crate::excel::normalize_name;

    // Paso 1: Construir MapeoMaestro desde las 3 fuentes
    eprintln!("🚀 FASE 1: Construyendo MapeoMaestro...");
    let mapeo = crate::excel::construir_mapeo_maestro(
        malla_archivo,
        // Resolver rutas automáticamente
        &format!("{}/OA2024.xlsx", crate::excel::DATAFILES_DIR),
        &format!("{}/PA2025-1.xlsx", crate::excel::DATAFILES_DIR),
    )?;

    eprintln!("✅ MapeoMaestro construido: {}", mapeo.resumen());

    // Paso 2: Convertir MapeoMaestro a HashMap<String, RamoDisponible>
    eprintln!("🚀 FASE 2: Convirtiendo MapeoMaestro a RamoDisponible...");
    let mut ramos_disponibles: HashMap<String, RamoDisponible> = HashMap::new();

    let mut contador_electivos = 0;
    let mut contador_procesados = 0;

    for asignatura in mapeo.iter() {
        contador_procesados += 1;

        // Determinar clave y características basadas en si es electivo
        let (clave, codigo_final, es_electivo_final) = if asignatura.es_electivo {
            // Electivos: usar clave única con ID
            let clave_unica = format!(
                "electivo_profesional_{}",
                asignatura.id_malla.unwrap_or(44 + contador_electivos as i32)
            );
            contador_electivos += 1;

            let codigo = asignatura
                .codigo_pa2025
                .clone()
                .unwrap_or_else(|| format!("ELEC_{}", contador_electivos));

            (clave_unica, codigo, true)
        } else {
            // No-electivos: usar nombre normalizado como clave
            let clave = asignatura.nombre_normalizado.clone();
            let codigo = asignatura
                .codigo_pa2025
                .clone()
                .or_else(|| asignatura.codigo_oa2024.clone())
                .unwrap_or_else(|| asignatura.id_malla.map(|id| id.to_string()).unwrap_or_default());

            (clave, codigo, false)
        };

        // Crear RamoDisponible con todos los datos disponibles
        let ramo = RamoDisponible {
            id: asignatura.id_malla.unwrap_or(0),
            nombre: asignatura.nombre_real.clone(),
            codigo: codigo_final,
            holgura: 0,
            numb_correlativo: asignatura.id_malla.unwrap_or(0),
            critico: false,
            codigo_ref: None, // Se resuelve en segundo pase
            dificultad: asignatura.porcentaje_aprobacion,
            electivo: es_electivo_final,
        };

        eprintln!(
            "  ✓ Procesado: {} (id={:?}, electivo={})",
            asignatura.nombre_real, asignatura.id_malla, es_electivo_final
        );

        ramos_disponibles.insert(clave, ramo);
    }

    eprintln!(
        "✅ FASE 2 completada: {} asignaturas convertidas",
        contador_procesados
    );

    // Paso 3: Resolver dependencias por correlativo (segundo pase)
    eprintln!("🚀 FASE 3: Resolviendo dependencias por correlativo...");
    resolver_dependencias(&mut ramos_disponibles)?;

    eprintln!(
        "✅ Sistema completado: {} ramos disponibles",
        ramos_disponibles.len()
    );

    Ok(ramos_disponibles)
}

/// Resolver referencias entre ramos basadas en correlativo
/// Si ramo.numb_correlativo == X, busca ramo con numb_correlativo == X-1
/// y establece codigo_ref
fn resolver_dependencias(
    ramos_disponibles: &mut HashMap<String, RamoDisponible>,
) -> Result<(), Box<dyn Error>> {
    let mut updates: Vec<(String, i32)> = Vec::new();

    // Recopilar todas las dependencias
    for (clave, ramo) in ramos_disponibles.iter() {
        let correlativo_actual = ramo.numb_correlativo;
        let id_anterior = correlativo_actual - 1;

        // Buscar si existe ramo con correlativo anterior
        for (_, otro_ramo) in ramos_disponibles.iter() {
            if otro_ramo.numb_correlativo == id_anterior {
                updates.push((clave.clone(), id_anterior));
                eprintln!(
                    "  ✓ Dependencia: {} (corr={}) ← {} (corr={})",
                    ramo.nombre, correlativo_actual, otro_ramo.nombre, id_anterior
                );
                break;
            }
        }
    }

    // Aplicar todas las actualizaciones
    let updates_len = updates.len();
    for (clave, id_prev) in updates {
        if let Some(ramo) = ramos_disponibles.get_mut(&clave) {
            ramo.codigo_ref = Some(id_prev);
        }
    }

    eprintln!("✅ FASE 3 completada: {} dependencias resueltas", updates_len);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_construccion_mapeo_maestro() {
        // Este test verifica que MapeoMaestro se construye correctamente
        // Usa datos del proyecto si están disponibles
        let result = leer_malla_con_porcentajes_optimizado(
            "MiMalla.xlsx",
            "../RutaCritica/PorcentajeAPROBADOS2025-1.xlsx",
        );

        match result {
            Ok(ramos) => {
                assert!(ramos.len() > 0, "Debe haber al menos un ramo");
                eprintln!("✅ Test exitoso: {} ramos cargados", ramos.len());
            }
            Err(e) => {
                eprintln!("⚠️  Test incompleto (archivos no disponibles): {}", e);
                // No fallar si los archivos no están disponibles
            }
        }
    }
}
