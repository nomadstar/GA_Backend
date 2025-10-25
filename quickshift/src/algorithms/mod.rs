// Módulo raíz para algoritmos. Reexporta funciones desde submódulos.

mod extract;
mod pert;
mod conflict;
mod clique;

pub use extract::{get_ramo_critico, extract_data};
pub use pert::set_values_recursive;
pub use conflict::horarios_tienen_conflicto;
pub use clique::{find_max_weight_clique, get_clique_max_pond, get_clique_max_pond_with_prefs, get_clique_with_user_prefs};

// Public wrapper that applies user preferences and delegates to the
// clique-based scheduler implemented in `clique.rs` (which in turn
// uses `find_max_weight_clique`).
//
// Contract:
// - Inputs: lista_secciones, ramos_disponibles, params (api_json::InputParams)
// - Output: Vec of (Vec<(Seccion, priority)>, total_score)
// - Error modes: none; function delegates filtering/handling to the
//   underlying implementation and returns an empty Vec on no-solution.
//
// This wrapper exists so external code can call a single well-named
// function from the algorithms module without reaching into submodules.
// wrapper is provided by the `clique` submodule; reexported above.


// wrapper is defined public above and available to callers as
// `quickshift::algorithms::get_clique_with_user_prefs`.
