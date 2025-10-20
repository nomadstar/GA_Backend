use crate::rutacomoda::PathResult;

/// Return all paths that have the maximum reported `score` value.
/// Ties are included.
pub fn choose_best_paths_by_reported_score(paths: &[PathResult]) -> Vec<(Vec<String>, f64)> {
    let mut best: Vec<(Vec<String>, f64)> = Vec::new();
    let mut max_score: Option<f64> = None;

    for p in paths {
        let s = p.score;
        match max_score {
            None => {
                max_score = Some(s);
                best.clear();
                best.push((p.path.clone(), s));
            }
            Some(ms) => {
                if (s - ms).abs() < std::f64::EPSILON {
                    // treat as tie when equal within f64 epsilon
                    best.push((p.path.clone(), s));
                } else if s > ms {
                    max_score = Some(s);
                    best.clear();
                    best.push((p.path.clone(), s));
                }
            }
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_path(code: &str, score: f64) -> PathResult {
        PathResult {
            path: code.split(',').map(|s| s.trim().to_string()).collect(),
            score,
            total_credits: None,
            missing_prereqs: Vec::new(),
            sections_recommended: Vec::new(),
            metadata: None,
        }
    }

    #[test]
    fn test_choose_best_single() {
        let paths = vec![make_path("A,B,C", 10.0), make_path("D,E", 8.0)];
        let best = choose_best_paths_by_reported_score(&paths);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].1, 10.0);
    }

    #[test]
    fn test_choose_best_ties() {
        let paths = vec![
            make_path("A,B", 9.5),
            make_path("C,D", 9.5),
            make_path("E", 7.0),
        ];
        let best = choose_best_paths_by_reported_score(&paths);
        assert_eq!(best.len(), 2);
        let scores: Vec<f64> = best.iter().map(|(_, s)| *s).collect();
        assert!(scores.iter().all(|&x| (x - 9.5).abs() < std::f64::EPSILON));
    }
}
