use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

mod selector;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::env;

    #[test]
    fn test_load_paths_and_best_paths() {
        // create a temporary JSON file representing PathsOutput
        let tmp_dir = env::temp_dir();
        let file_path = tmp_dir.join("paths_output_test.json");

        let json = r#"
        {
            "paths": [
                { "path": ["A","B","C"], "score": 5.0, "total_credits": null, "missing_prereqs": [], "sections_recommended": [], "metadata": null },
                { "path": ["D","E"], "score": 8.5, "total_credits": null, "missing_prereqs": [], "sections_recommended": [], "metadata": null },
                { "path": ["F"], "score": 8.5, "total_credits": null, "missing_prereqs": [], "sections_recommended": [], "metadata": null }
            ]
        }
        "#;

        let mut f = File::create(&file_path).expect("create tmp file");
        f.write_all(json.as_bytes()).expect("write test json");

        let loaded = load_paths_from_file(&file_path).expect("load paths");
        assert_eq!(loaded.paths.len(), 3);

        let best = best_paths(&loaded);
        // two best with score 8.5
        assert_eq!(best.len(), 2);
        let scores: Vec<f64> = best.iter().map(|(_, s)| *s).collect();
        assert!(scores.iter().all(|&x| (x - 8.5).abs() < std::f64::EPSILON));
    }
}
