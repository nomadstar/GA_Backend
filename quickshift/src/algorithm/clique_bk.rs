use std::time::Instant;

// Implementación de Bron–Kerbosch con pivote y bitsets. Exporta una función
// pequeña que recibe la lista de vecinos como bitsets y pesos por vértice y
// devuelve las posiciones (índices) de los vértices que componen la mejor
// clique encontrada dentro del presupuesto de tiempo.

#[inline]
fn bitset_is_empty(bs: &Vec<u64>) -> bool { bs.iter().all(|w| *w == 0) }

#[inline]
fn bitset_count(bs: &Vec<u64>) -> usize { bs.iter().map(|w| w.count_ones() as usize).sum() }

#[inline]
fn bitset_copy(a: &Vec<u64>) -> Vec<u64> { a.clone() }

#[inline]
fn bitset_and(a: &Vec<u64>, b: &Vec<u64>) -> Vec<u64> { a.iter().zip(b.iter()).map(|(x,y)| x & y).collect() }

#[inline]
fn bitset_or(a: &Vec<u64>, b: &Vec<u64>) -> Vec<u64> { a.iter().zip(b.iter()).map(|(x,y)| x | y).collect() }

#[inline]
fn bitset_not(a: &Vec<u64>) -> Vec<u64> { a.iter().map(|x| !*x).collect() }

// Itera sobre los índices con bit 1 en el bitset. Llama f(idx) y si retorna false corta.
fn for_each_bit<F: FnMut(usize) -> bool>(bs: &Vec<u64>, mut f: F) {
    for (word_idx, &word) in bs.iter().enumerate() {
        let mut w = word;
        while w != 0 {
            let tz = w.trailing_zeros() as usize;
            let idx = word_idx * 64 + tz;
            if !f(idx) { return; }
            w &= w - 1;
        }
    }
}

pub fn bk_find_max_weight_clique(
    neigh: &Vec<Vec<u64>>,
    weights: &Vec<i32>,
    max_size: usize,
    budget_ms: u128,
) -> Vec<usize> {
    let n = neigh.len();
    let words = if n == 0 { 0 } else { (n + 63) / 64 };
    let mut p = vec![0u64; words];
    for i in 0..n { let w = i / 64; let b = i % 64; p[w] |= 1u64 << b; }
    let x = vec![0u64; words];
    let mut r: Vec<usize> = Vec::new();

    let start = Instant::now();
    let mut best_clique: Vec<usize> = Vec::new();
    let mut best_score: i64 = i64::MIN;
    let mut aborted = false;

    fn bk_rec(
        neigh: &Vec<Vec<u64>>,
        weights: &Vec<i32>,
        max_size: usize,
        start: Instant,
        budget_ms: u128,
        r: &mut Vec<usize>,
        p: &Vec<u64>,
        x: &Vec<u64>,
        best_clique: &mut Vec<usize>,
        best_score: &mut i64,
        aborted: &mut bool,
    ) {
        if *aborted { return; }
        if start.elapsed().as_millis() > budget_ms { *aborted = true; return; }
        if bitset_is_empty(p) && bitset_is_empty(x) {
            if r.len() <= max_size {
                let score: i64 = r.iter().map(|&i| weights[i] as i64).sum();
                if score > *best_score { *best_score = score; *best_clique = r.clone(); }
            } else {
                let mut tmp = r.clone();
                tmp.sort_by_key(|&i| -(weights[i]));
                let score: i64 = tmp.iter().take(max_size).map(|&i| weights[i] as i64).sum();
                if score > *best_score { *best_score = score; *best_clique = tmp.into_iter().take(max_size).collect(); }
            }
            return;
        }

        let p_union_x = bitset_or(p, x);
        let mut u_opt: Option<usize> = None;
        {
            let mut best_cnt = 0usize;
            for_each_bit(&p_union_x, |u| {
                let inter = bitset_and(&neigh[u], p);
                let cnt = bitset_count(&inter);
                if cnt > best_cnt { best_cnt = cnt; u_opt = Some(u); }
                true
            });
        }

        let mut candidates = if let Some(u) = u_opt {
            let mut not_nu = bitset_not(&neigh[u]);
            bitset_and(p, &not_nu)
        } else { bitset_copy(p) };

        let mut cand_vertices: Vec<usize> = Vec::new();
        for_each_bit(&candidates, |v| { cand_vertices.push(v); true });

        for v in cand_vertices {
            if start.elapsed().as_millis() > budget_ms { *aborted = true; return; }
            r.push(v);
            let p_new = bitset_and(p, &neigh[v]);
            let x_new = bitset_and(x, &neigh[v]);
            bk_rec(neigh, weights, max_size, start, budget_ms, r, &p_new, &x_new, best_clique, best_score, aborted);
            r.pop();
        }
    }

    bk_rec(neigh, weights, max_size, start, budget_ms, &mut r, &p, &x, &mut best_clique, &mut best_score, &mut aborted);
    best_clique
}
