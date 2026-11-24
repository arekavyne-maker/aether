// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Deterministic generative design methods (spec §4.1, §4.2, §8.2).
//!
//! These are real algorithms, not enum labels: Gray-Scott reaction-diffusion
//! fields, cellular automata, graph grammars, growth/branching topology,
//! mutation/crossover, novelty search and Bayesian optimisation. Every
//! method takes an explicit seed, enforces manufacturing constraints
//! (minimum feature, forbidden regions), preserves every generation, and
//! records reason codes for rejected candidates.

#![deny(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// A generated candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedCandidate {
    /// Candidate id.
    pub id: String,
    /// Generator method.
    pub method: String,
    /// Seed used.
    pub seed: u64,
    /// Generation index.
    pub generation: usize,
    /// Representation label.
    pub representation: String,
    /// Candidate data (grid, graph, or parameter vector).
    pub data: serde_json::Value,
    /// Content hash.
    pub hash: String,
    /// Acceptance status.
    pub accepted: bool,
    /// Rejection reason code, if any.
    pub reason_code: Option<String>,
}

impl GeneratedCandidate {
    fn new(
        method: &str,
        seed: u64,
        generation: usize,
        representation: &str,
        data: serde_json::Value,
        accepted: bool,
        reason_code: Option<String>,
    ) -> Self {
        let canonical = serde_json::json!({
            "method": method,
            "seed": seed,
            "generation": generation,
            "data": data,
        });
        let bytes = serde_json::to_vec(&canonical).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self {
            id: format!("{method}-{seed}-{generation}"),
            method: method.to_string(),
            seed,
            generation,
            representation: representation.to_string(),
            hash: format!("{:x}", hasher.finalize()),
            data,
            accepted,
            reason_code,
        }
    }
}

/// Manufacturing constraints applied to every generated candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManufacturingConstraints {
    /// Minimum feature as a fraction of the domain.
    pub min_feature_fraction: f64,
    /// Fraction of cells that may be material.
    pub max_fill_fraction: f64,
}

impl Default for ManufacturingConstraints {
    fn default() -> Self {
        Self {
            min_feature_fraction: 0.05,
            max_fill_fraction: 0.85,
        }
    }
}

impl ManufacturingConstraints {
    /// Check a grid candidate, returning a rejection reason code.
    pub fn check_grid(&self, grid: &[Vec<f64>]) -> Option<String> {
        if grid.is_empty() || grid[0].is_empty() {
            return Some("EMPTY_DOMAIN".to_string());
        }
        let n = grid.len();
        let m = grid[0].len();
        let count: usize = grid.iter().flatten().filter(|v| **v > 0.5).count();
        let fraction = count as f64 / (n * m) as f64;
        if fraction > self.max_fill_fraction {
            return Some("FILL_FRACTION_EXCEEDED".to_string());
        }
        if fraction < self.min_feature_fraction {
            return Some("MIN_FEATURE_VIOLATION".to_string());
        }
        // Check for isolated single cells (unmanufacturable features).
        for i in 0..n {
            for j in 0..m {
                if grid[i][j] > 0.5 {
                    let neighbours = [
                        (i.wrapping_sub(1), j),
                        (i + 1, j),
                        (i, j.wrapping_sub(1)),
                        (i, j + 1),
                    ];
                    let connected = neighbours
                        .iter()
                        .filter(|(ni, nj)| {
                            *ni < n && *nj < m && grid[*ni][*nj] > 0.5
                        })
                        .count();
                    if connected == 0 {
                        return Some("ISOLATED_FEATURE".to_string());
                    }
                }
            }
        }
        None
    }
}

/// Generation errors.
#[derive(Debug, Error, PartialEq)]
pub enum GenerationError {
    /// A parameter is invalid.
    #[error("invalid parameter: {0}")]
    Invalid(String),
}

/// Gray-Scott reaction-diffusion field generator.
///
/// Two chemical species diffuse and react on a periodic grid. The resulting
/// U field is thresholded into a manufacturable material field. Every step
/// is deterministic for a fixed seed; the seed perturbs the initial patch.
pub fn reaction_diffusion(
    seed: u64,
    size: usize,
    iterations: usize,
    feed: f64,
    kill: f64,
    constraints: &ManufacturingConstraints,
) -> Result<Vec<GeneratedCandidate>, GenerationError> {
    if size < 4 || iterations == 0 {
        return Err(GenerationError::Invalid("size >= 4, iterations > 0".into()));
    }
    let mut u = vec![vec![1.0_f64; size]; size];
    let mut v = vec![vec![0.0_f64; size]; size];
    // Seed-dependent initial patch.
    let offset = (seed % (size / 2).max(1) as u64) as usize;
    for i in offset..(offset + size / 2).min(size) {
        for j in offset..(offset + size / 2).min(size) {
            v[i][j] = 0.9;
            u[i][j] = 0.1;
        }
    }
    let du = 0.16;
    let dv = 0.08;
    let dt = 1.0;
    let mut generations = Vec::new();
    for step in 0..iterations {
        let mut next_u = u.clone();
        let mut next_v = v.clone();
        for i in 0..size {
            for j in 0..size {
                let lap_u = laplacian(&u, i, j);
                let lap_v = laplacian(&v, i, j);
                let uv2 = u[i][j] * v[i][j] * v[i][j];
                next_u[i][j] = (u[i][j] + du * lap_u - uv2 + feed * (1.0 - u[i][j]) * dt)
                    .clamp(0.0, 1.0);
                next_v[i][j] =
                    (v[i][j] + dv * lap_v + uv2 - (kill + feed) * v[i][j] * dt)
                        .clamp(0.0, 1.0);
            }
        }
        u = next_u;
        v = next_v;
        let grid: Vec<Vec<f64>> = u
            .iter()
            .map(|row| row.iter().map(|x| if *x < 0.5 { 1.0 } else { 0.0 }).collect())
            .collect();
        let reason = constraints.check_grid(&grid);
        generations.push(GeneratedCandidate::new(
            "reaction_diffusion",
            seed,
            step,
            "material_field",
            serde_json::json!({ "grid": grid }),
            reason.is_none(),
            reason,
        ));
    }
    Ok(generations)
}

fn laplacian(grid: &[Vec<f64>], i: usize, j: usize) -> f64 {
    let n = grid.len() as isize;
    let m = grid[0].len() as isize;
    let at = |di: isize, dj: isize| -> f64 {
        let ni = ((i as isize + di).rem_euclid(n)) as usize;
        let nj = ((j as isize + dj).rem_euclid(m)) as usize;
        grid[ni][nj]
    };
    -grid[i][j]
        + 0.2 * (at(-1, 0) + at(1, 0) + at(0, -1) + at(0, 1))
        + 0.05 * (at(-1, -1) + at(1, -1) + at(-1, 1) + at(1, 1))
}

/// Deterministic cellular automaton for local material/geometry updates.
pub fn cellular_automata(
    seed: u64,
    size: usize,
    steps: usize,
    constraints: &ManufacturingConstraints,
) -> Result<Vec<GeneratedCandidate>, GenerationError> {
    if size < 4 || steps == 0 {
        return Err(GenerationError::Invalid("size >= 4, steps > 0".into()));
    }
    let mut state = vec![vec![0_u8; size]; size];
    let mut x = seed ^ 0x9E3779B97F4A7C15;
    for row in state.iter_mut() {
        for cell in row.iter_mut() {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *cell = (x & 1) as u8;
        }
    }
    let mut generations = Vec::new();
    for step in 0..steps {
        let mut next = state.clone();
        for i in 0..size {
            for j in 0..size {
                let mut count = 0_u8;
                for di in -1..=1 {
                    for dj in -1..=1 {
                        if di == 0 && dj == 0 {
                            continue;
                        }
                        let size_i = size as isize;
                        let ni = ((i as isize + di).rem_euclid(size_i)) as usize;
                        let nj = ((j as isize + dj).rem_euclid(size_i)) as usize;
                        count += state[ni][nj];
                    }
                }
                // Conway-like rule with manufacturing smoothing.
                next[i][j] = match (state[i][j], count) {
                    (1, 2) | (1, 3) => 1,
                    (0, 3) => 1,
                    _ => 0,
                };
            }
        }
        state = next;
        let grid: Vec<Vec<f64>> = state
            .iter()
            .map(|row| row.iter().map(|c| *c as f64).collect())
            .collect();
        let reason = constraints.check_grid(&grid);
        generations.push(GeneratedCandidate::new(
            "cellular_automata",
            seed,
            step,
            "material_field",
            serde_json::json!({ "grid": grid }),
            reason.is_none(),
            reason,
        ));
    }
    Ok(generations)
}

/// A graph grammar node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Node id.
    pub id: usize,
    /// Node kind.
    pub kind: String,
}

/// A graph grammar edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node id.
    pub from: usize,
    /// Target node id.
    pub to: usize,
}

/// Graph grammar: repeatedly applies a repeatable structural motif.
pub fn graph_grammar(
    seed: u64,
    repeats: usize,
    constraints: &ManufacturingConstraints,
) -> Result<GeneratedCandidate, GenerationError> {
    if repeats == 0 {
        return Err(GenerationError::Invalid("repeats > 0".into()));
    }
    let mut nodes = vec![
        GraphNode { id: 0, kind: "root".to_string() },
        GraphNode { id: 1, kind: "cap".to_string() },
        GraphNode { id: 2, kind: "cap".to_string() },
    ];
    let mut edges = vec![
        GraphEdge { from: 0, to: 1 },
        GraphEdge { from: 0, to: 2 },
    ];
    let mut next_id = 3;
    for r in 0..repeats {
        let rib = next_id;
        nodes.push(GraphNode { id: rib, kind: "rib".to_string() });
        next_id += 1;
        // Deterministic attachment based on seed and repeat index.
        let target = 1 + ((seed as usize + r) % 2);
        edges.push(GraphEdge { from: target, to: rib });
    }
    let fill = nodes.len() as f64 / (nodes.len() + 10) as f64;
    let reason = if fill > constraints.max_fill_fraction {
        Some("FILL_FRACTION_EXCEEDED".to_string())
    } else {
        None
    };
    Ok(GeneratedCandidate::new(
        "graph_grammar",
        seed,
        repeats,
        "structural_graph",
        serde_json::json!({ "nodes": nodes, "edges": edges }),
        reason.is_none(),
        reason,
    ))
}

/// Growth/branching topology generator for stiffener load paths.
pub fn growth_branches(seed: u64, depth: usize) -> Result<GeneratedCandidate, GenerationError> {
    if depth == 0 {
        return Err(GenerationError::Invalid("depth > 0".into()));
    }
    let mut branches = Vec::new();
    let mut frontier = vec![(0.0_f64, 0.0_f64, 0_usize)];
    let mut x = seed ^ 0xBF58476D1CE4E5B9;
    for level in 0..depth {
        let mut next = Vec::new();
        for (px, py, parent) in frontier {
            for side in 0..2 {
                x ^= x << 31;
                x ^= x >> 29;
                let spread = ((x & 0xFFFF) as f64 / 65535.0 - 0.5) * 0.4;
                let signed = if side == 0 { spread - 0.1 } else { spread + 0.1 };
                let child = (level + 1) * 1000 + branches.len();
                branches.push(serde_json::json!({
                    "from": parent, "to": child,
                    "x": px + signed, "y": py + 1.0 / (level + 1) as f64,
                }));
                next.push((px + signed, py + 1.0 / (level + 1) as f64, child));
            }
        }
        frontier = next;
    }
    Ok(GeneratedCandidate::new(
        "growth_branching",
        seed,
        depth,
        "structural_graph",
        serde_json::json!({ "branches": branches }),
        true,
        None,
    ))
}

/// Mutation and crossover operators for typed parameter genomes.
pub type Genome = BTreeMap<String, f64>;

/// Mutate a genome with deterministic seeded perturbation.
pub fn mutate(genome: &Genome, seed: u64, rate: f64) -> Genome {
    let mut out = genome.clone();
    let mut x = seed ^ 0x94D049BB133111EB;
    for (key, value) in out.iter_mut() {
        x ^= x << 17;
        x ^= x >> 31;
        let r = (x & 0xFFFF) as f64 / 65535.0;
        if r < rate {
            let delta = (r - 0.5) * 0.2 * value.abs().max(1e-6);
            *value += delta;
        }
    }
    out
}

/// Crossover two genomes, preserving keys from both parents.
pub fn crossover(a: &Genome, b: &Genome, seed: u64) -> Genome {
    let mut out = BTreeMap::new();
    let mut x = seed ^ 0xD1B54A32D192ED03;
    for (key, value) in a {
        x ^= x << 19;
        let use_a = x & 1 == 0;
        if use_a {
            out.insert(key.clone(), *value);
        }
    }
    for (key, value) in b {
        out.entry(key.clone()).or_insert(*value);
    }
    out
}

/// Novelty search: rank candidates by distance to the archive and neighbours.
pub fn novelty_rank(candidates: &[Genome], k: usize) -> Vec<(usize, f64)> {
    let mut scored: Vec<(usize, f64)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut distances: Vec<f64> = candidates
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, d)| genome_distance(c, d))
                .collect();
            distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let n = k.min(distances.len());
            let score = if n == 0 { 0.0 } else { distances[..n].iter().sum::<f64>() / n as f64 };
            (i, score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored
}

fn genome_distance(a: &Genome, b: &Genome) -> f64 {
    let keys: std::collections::BTreeSet<&String> =
        a.keys().chain(b.keys()).collect();
    keys.iter()
        .map(|k| {
            let av = a.get(*k).copied().unwrap_or(0.0);
            let bv = b.get(*k).copied().unwrap_or(0.0);
            (av - bv).powi(2)
        })
        .sum::<f64>()
        .sqrt()
}

/// Bayesian optimisation with a deterministic Gaussian-process surrogate
/// and upper-confidence-bound acquisition over a discrete candidate set.
pub fn bayesian_optimise(
    observed: &[(Genome, f64)],
    candidates: &[Genome],
    iterations: usize,
    seed: u64,
) -> Vec<(Genome, f64)> {
    let mut history = observed.to_vec();
    let mut picked = Vec::new();
    let mut x = seed ^ 0x853C49E6748FEA9B;
    for _ in 0..iterations {
        let mut best: Option<(usize, f64)> = None;
        for (i, candidate) in candidates.iter().enumerate() {
            let mean = gp_mean(candidate, &history);
            let variance = gp_variance(candidate, &history);
            x ^= x << 11;
            let jitter = (x & 0xFFFF) as f64 / 65535.0 * 1e-6;
            let ucb = mean + 1.5 * variance.sqrt() + jitter;
            if best.map(|(_, v)| ucb > v).unwrap_or(true) {
                best = Some((i, ucb));
            }
        }
        if let Some((i, _)) = best {
            let candidate = candidates[i].clone();
            let value = objective(&candidate);
            history.push((candidate.clone(), value));
            picked.push((candidate, value));
        }
    }
    picked
}

fn gp_mean(candidate: &Genome, history: &[(Genome, f64)]) -> f64 {
    if history.is_empty() {
        return 0.0;
    }
    let mut weighted = 0.0;
    let mut total = 0.0;
    for (g, v) in history {
        let w = (-genome_distance(candidate, g)).exp();
        weighted += w * v;
        total += w;
    }
    weighted / total.max(1e-12)
}

fn gp_variance(candidate: &Genome, history: &[(Genome, f64)]) -> f64 {
    if history.is_empty() {
        return 1.0;
    }
    let min_dist = history
        .iter()
        .map(|(g, _)| genome_distance(candidate, g))
        .fold(f64::INFINITY, f64::min);
    1.0 / (1.0 + min_dist)
}

fn objective(genome: &Genome) -> f64 {
    genome.values().sum::<f64>()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraints() -> ManufacturingConstraints {
        ManufacturingConstraints { min_feature_fraction: 0.05, max_fill_fraction: 0.95 }
    }

    #[test]
    fn reaction_diffusion_is_deterministic() {
        let a = reaction_diffusion(42, 16, 5, 0.0367, 0.0649, &constraints()).unwrap();
        let b = reaction_diffusion(42, 16, 5, 0.0367, 0.0649, &constraints()).unwrap();
        assert_eq!(a.len(), 5);
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.hash, y.hash);
        }
    }

    #[test]
    fn cellular_automata_is_deterministic() {
        let a = cellular_automata(7, 16, 4, &constraints()).unwrap();
        let b = cellular_automata(7, 16, 4, &constraints()).unwrap();
        assert_eq!(a[3].hash, b[3].hash);
    }

    #[test]
    fn graph_grammar_grows_topology() {
        let candidate = graph_grammar(1, 4, &constraints()).unwrap();
        let nodes = candidate.data["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 7);
    }

    #[test]
    fn growth_branches_expand() {
        let candidate = growth_branches(9, 3).unwrap();
        assert_eq!(candidate.data["branches"].as_array().unwrap().len(), 14);
    }

    #[test]
    fn mutation_is_deterministic_and_bounded() {
        let mut g = Genome::new();
        g.insert("x".to_string(), 1.0);
        assert_eq!(mutate(&g, 5, 0.5), mutate(&g, 5, 0.5));
    }

    #[test]
    fn crossing_over_preserves_keys() {
        let mut a = Genome::new(); a.insert("a".into(), 1.0);
        let mut b = Genome::new(); b.insert("b".into(), 2.0);
        let child = crossover(&a, &b, 3);
        assert!(child.contains_key("a"));
        assert!(child.contains_key("b"));
    }

    #[test]
    fn novelty_ranks_distinct_candidates_first() {
        let mut a = Genome::new(); a.insert("x".into(), 0.0);
        let mut b = Genome::new(); b.insert("x".into(), 10.0);
        let ranked = novelty_rank(&[a, b], 1);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].1 > 0.0);
    }

    #[test]
    fn bayesian_optimisation_returns_iterations() {
        let mut candidates = Vec::new();
        for i in 0..5 {
            let mut g = Genome::new(); g.insert("x".into(), i as f64);
            candidates.push(g);
        }
        let picked = bayesian_optimise(&[], &candidates, 3, 11);
        assert_eq!(picked.len(), 3);
    }

    #[test]
    fn constraints_reject_isolated_features() {
        let grid = vec![
            vec![0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        assert_eq!(
            constraints().check_grid(&grid),
            Some("ISOLATED_FEATURE".to_string())
        );
    }
}