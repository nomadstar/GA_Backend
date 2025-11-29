//! Caché simple en memoria para lecturas de Excel costosas
//!
//! Proporciona get_prereqs_cached(malla_name) -> Arc<HashMap<String, Vec<String>>>
//! que intenta devolver la tabla de prerequisitos ya parseada para la malla indicada.

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicUsize, Ordering};

// Tipo concreto esperado por `leer_prerequisitos`
type PrMap = HashMap<String, Vec<String>>;

// Caché global: mapa malla_path -> Arc<PrMap>
static PREREQ_CACHE: OnceLock<Mutex<HashMap<String, Arc<PrMap>>>> = OnceLock::new();

// Estadísticas simples de caché (hits / misses)
static PREREQ_CACHE_HITS: OnceLock<AtomicUsize> = OnceLock::new();
static PREREQ_CACHE_MISSES: OnceLock<AtomicUsize> = OnceLock::new();

/// Devuelve los prerequisitos de la malla solicitada, usando el caché en memoria
/// si está disponible; en caso contrario lee y almacena el resultado.
///
/// Key notes:
/// - la clave usada en el caché es la "malla_path" resuelta a string (si se
///   puede), de modo que distintas representaciones de la misma ruta no
///   duplican la entrada cuando se pasan exactamente la misma ruta.
/// - la función mantiene un Mutex muy corto (bloqueo breve) para controlar la
///   inserción en la tabla; el resultado se devuelve como Arc para compartirlo
///   sin clonaciones costosas.
pub fn get_prereqs_cached(malla_name: &str) -> Result<Arc<PrMap>, Box<dyn Error>> {
    let cache = PREREQ_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let hits = PREREQ_CACHE_HITS.get_or_init(|| AtomicUsize::new(0));
    let misses = PREREQ_CACHE_MISSES.get_or_init(|| AtomicUsize::new(0));
    // Resolve path (intento práctico: usar resolve_datafile_paths si funciona)
    let malla_pathbuf = match crate::excel::resolve_datafile_paths(malla_name) {
        Ok((m, _, _)) => m,
        Err(_) => std::path::PathBuf::from(malla_name.to_string()),
    };
    let key = malla_pathbuf.to_str().unwrap_or(malla_name).to_string();

    // Primera: intentar devolver del caché si ya existe
    {
        let guard = cache.lock().expect("prereq cache mutex poisoned");
        if let Some(existing) = guard.get(&key) {
            hits.fetch_add(1, Ordering::SeqCst);
            return Ok(Arc::clone(existing));
        }
    }

    // Si no está en caché: leer desde disco usando la función existente
    let path_str = key.clone();
    match crate::excel::leer_prerequisitos(&path_str) {
        Ok(map) => {
            misses.fetch_add(1, Ordering::SeqCst);
            let arc = Arc::new(map);
            let mut guard = cache.lock().expect("prereq cache mutex poisoned");
            // Guardar con la clave "key"
            guard.insert(key, Arc::clone(&arc));
            Ok(arc)
        }
        Err(e) => Err(e),
    }
}

/// Devuelve estadísticas simples de la caché: (hits, misses, entries)
pub fn get_prereq_cache_stats() -> (usize, usize, usize) {
    let cache = PREREQ_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let hits = PREREQ_CACHE_HITS.get_or_init(|| AtomicUsize::new(0));
    let misses = PREREQ_CACHE_MISSES.get_or_init(|| AtomicUsize::new(0));
    let guard = cache.lock().expect("prereq cache mutex poisoned");
    (hits.load(Ordering::SeqCst), misses.load(Ordering::SeqCst), guard.len())
}
