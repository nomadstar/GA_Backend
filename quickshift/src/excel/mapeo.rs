/// Módulo para mapeo universal entre los 3 sistemas de códigos:
/// - Malla2020: ID numérico + Nombre
/// - OA2024: Código alfabético (CBF1000, CIT2109, etc.) + Nombre
/// - PA2025-1: Código alfabético + Nombre + Porcentaje
///
/// Clave universal: NOMBRE NORMALIZADO (único para cada asignatura)
///
/// Estructura (ejemplo):
/// ```text
/// NOMBRE_NORMALIZADO
///   - ID Malla (ej: 7)
///   - Código OA2024 (ej: CBM1003)
///   - Código PA2025-1 (ej: CBM1003)
///   - Porcentaje (ej: 53.13%)
///   - Es Electivo (true/false)
/// ```

use std::collections::HashMap;

/// Estructura que representa la información unificada de una asignatura
#[derive(Clone, Debug)]
pub struct MapeoAsignatura {
    pub nombre_normalizado: String,
    pub nombre_real: String,
    pub id_malla: Option<i32>,
    pub codigo_oa2024: Option<String>,
    pub codigo_pa2025: Option<String>,
    pub porcentaje_aprobacion: Option<f64>,
    pub es_electivo: bool,
}

impl MapeoAsignatura {
    pub fn new(nombre_normalizado: String, nombre_real: String) -> Self {
        MapeoAsignatura {
            nombre_normalizado,
            nombre_real,
            id_malla: None,
            codigo_oa2024: None,
            codigo_pa2025: None,
            porcentaje_aprobacion: None,
            es_electivo: false,
        }
    }
}

/// Estructura maestra que contiene todos los mapeos
pub struct MapeoMaestro {
    /// Clave: nombre_normalizado
    /// Valor: información unificada de la asignatura
    pub asignaturas: HashMap<String, MapeoAsignatura>,
}

impl MapeoMaestro {
    pub fn new() -> Self {
        MapeoMaestro {
            asignaturas: HashMap::new(),
        }
    }

    /// Agregar o actualizar información de una asignatura
    pub fn add_asignatura(&mut self, mapeo: MapeoAsignatura) {
        self.asignaturas.insert(mapeo.nombre_normalizado.clone(), mapeo);
    }

    /// Buscar por nombre normalizado
    pub fn get(&self, nombre_norm: &str) -> Option<&MapeoAsignatura> {
        self.asignaturas.get(nombre_norm)
    }

    /// Buscar por código OA2024
    pub fn get_by_codigo_oa(&self, codigo: &str) -> Option<&MapeoAsignatura> {
        self.asignaturas.values().find(|a| a.codigo_oa2024.as_deref() == Some(codigo))
    }

    /// Buscar por código PA2025-1
    pub fn get_by_codigo_pa(&self, codigo: &str) -> Option<&MapeoAsignatura> {
        self.asignaturas.values().find(|a| a.codigo_pa2025.as_deref() == Some(codigo))
    }

    /// Buscar por ID Malla
    pub fn get_by_id_malla(&self, id: i32) -> Option<&MapeoAsignatura> {
        self.asignaturas.values().find(|a| a.id_malla == Some(id))
    }

    /// Obtener todas las asignaturas
    pub fn iter(&self) -> std::collections::hash_map::Values<'_, String, MapeoAsignatura> {
        self.asignaturas.values()
    }

    /// Contar asignaturas
    pub fn len(&self) -> usize {
        self.asignaturas.len()
    }

    /// Obtener resumen
    pub fn resumen(&self) -> String {
        let total = self.asignaturas.len();
        let con_oa = self.asignaturas.values().filter(|a| a.codigo_oa2024.is_some()).count();
        let con_pa = self.asignaturas.values().filter(|a| a.codigo_pa2025.is_some()).count();
        let electivos = self.asignaturas.values().filter(|a| a.es_electivo).count();
        
        format!(
            "MAPEO MAESTRO: {} asignaturas totales | {} con OA2024 | {} con PA2025-1 | {} electivos",
            total, con_oa, con_pa, electivos
        )
    }
}

impl Default for MapeoMaestro {
    fn default() -> Self {
        Self::new()
    }
}

