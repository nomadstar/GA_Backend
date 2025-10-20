use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub mod selector;

#[derive(Debug, Serialize, Deserialize)]
pub struct SectionChoice {
    pub codigo: String,
    pub seccion: String,
    pub horario: Vec<String>,
    pub profesor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathResult {
    pub path: Vec<String>,
    pub score: f64,
    pub total_credits: Option<u32>,
    pub missing_prereqs: Vec<String>,
    pub sections_recommended: Vec<SectionChoice>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathsOutput {
    pub paths: Vec<PathResult>,
}

/// Load PathsOutput from a JSON file produced by Ruta crítica
pub fn load_paths_from_file<P: AsRef<Path>>(p: P) -> Result<PathsOutput, Box<dyn std::error::Error>> {
    let s = fs::read_to_string(p)?;
    let v: PathsOutput = serde_json::from_str(&s)?;
    Ok(v)
}

/// Return all best paths according to selector::choose_best_paths
pub fn best_paths(paths_output: &PathsOutput) -> Vec<(Vec<String>, f64)> {
    // collect paths and scores
    // simply delegate to selector which uses PathResult.score
    selector::choose_best_paths_by_reported_score(&paths_output.paths)
}
