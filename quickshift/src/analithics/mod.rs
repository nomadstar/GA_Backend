pub mod db;
pub mod queries;
pub mod insertions;
pub mod jsonparsing;

pub use db::init_db;
pub use insertions::{log_query, save_report};
pub use queries::{ramos_mas_pasados, ranking_por_estudiante, count_users, filtros_mas_solicitados, ramos_mas_recomendados, tasa_aprobacion_por_ramo, promedio_ranking_y_stddev, horarios_mas_ocupados};
pub use queries::{profesores_y_cursos, cursos_por_malla, horarios_mas_recomendados};
