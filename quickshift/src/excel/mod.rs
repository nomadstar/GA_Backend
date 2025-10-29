//! Módulo `excel` dividido en submódulos para mantener el código organizado.
//!
//! Submódulos:
//! - `io`: helpers y utilidades para lectura/parseo de Excel
//! - `malla`: lectura de mallas curriculares
//! - `porcentajes`: lectura de porcentajes/aprobados
//! - `oferta`: lectura de oferta académica
//! - `asignatura`: búsqueda de "Asignatura" por "Nombre Asignado"

/// Helpers de IO y utilidades para parsing de Excel
mod io;

/// Lectura de malla curricular: `leer_malla_excel`
mod malla;

/// Lectura de porcentajes/aprobados: `leer_porcentajes_aprobados`
mod porcentajes;

/// Lectura de oferta académica: `leer_oferta_academica_excel`
mod oferta;

/// Búsqueda de "Asignatura" a partir de "Nombre Asignado": `asignatura_from_nombre`
mod asignatura;

// Re-exports: helpers de IO son internos al crate; exponemos sólo las funciones de alto nivel
// helpers internos — no exportarlos públicamente
pub(crate) use io::{normalize_header, column_letters_to_index, cell_to_string, data_to_string, read_sheet_via_zip};
// funciones de alto nivel que sí usa `algorithm`
pub use malla::leer_malla_excel;
pub use porcentajes::leer_porcentajes_aprobados;
pub use oferta::leer_oferta_academica_excel;
pub use asignatura::asignatura_from_nombre;
