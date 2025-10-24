// Módulo raíz para algoritmos. Reexporta funciones desde submódulos.

pub mod extract;
pub mod pert;
pub mod conflict;
pub mod clique;

pub use extract::{get_ramo_critico, extract_data};
pub use pert::set_values_recursive;
pub use conflict::horarios_tienen_conflicto;
pub use clique::{find_max_weight_clique, get_clique_max_pond, get_clique_max_pond_with_prefs};
