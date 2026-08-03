//! Sugiyama-lite layered layout: longest-path layering over a DAG (cycles
//! broken by DFS), barycenter ordering sweeps, then gap-respecting coordinate
//! assignment. Operates on indices; the caller maps NodeIds.

use crate::ast::LayoutDir;

pub const LAYER_GAP: f32 = 70.0;
pub const NODE_GAP: f32 = 40.0;

/// `sizes[i]` is the world size of item i; `seed(i)` is its previous position
/// (used to stabilize ordering across relayouts). Returns center positions.
pub fn layout(
    sizes: &[[f32; 2]],
    edges: &[(usize, usize)],
    dir: LayoutDir,
    seed: impl Fn(usize) -> Option<[f32; 2]>,
) -> Vec<[f32; 2]> {
    let n = sizes.len();
    if n == 0 {
        return Vec::new();
    }
    // Axis mapping: primary = flow direction, cross = the other one.
    let (pi, ci) = match dir {
        LayoutDir::TopBottom => (1usize, 0usize),
        LayoutDir::LeftRight => (0usize, 1usize),
    };

    let dag = break_cycles(n, edges);
    let layers = assign_layers(n, &dag);
    let layer_count = layers.iter().copied().max().unwrap_or(0) + 1;

    // Group nodes per layer, initially ordered by seed cross-coordinate (falling
    // back to index order) so unrelated edits don't shuffle rows.
    let mut by_layer: Vec<Vec<usize>> = vec![Vec::new(); layer_count];
    for (v, &l) in layers.iter().enumerate() {
        by_layer[l].push(v);
    }
    for row in &mut by_layer {
        row.sort_by(|&a, &b| {
            let ka = seed(a).map(|p| p[ci]).unwrap_or(a as f32 * 1e-3);
            let kb = seed(b).map(|p| p[ci]).unwrap_or(b as f32 * 1e-3);
            ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Barycenter ordering sweeps.
    let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in &dag {
        succs[a].push(b);
        preds[b].push(a);
    }
    let order_of = |by_layer: &Vec<Vec<usize>>| {
        let mut pos = vec![0usize; n];
        for row in by_layer {
            for (i, &v) in row.iter().enumerate() {
                pos[v] = i;
            }
        }
        pos
    };
    for sweep in 0..4 {
        let downward = sweep % 2 == 0;
        let pos = order_of(&by_layer);
        let range: Vec<usize> = if downward {
            (0..layer_count).collect()
        } else {
            (0..layer_count).rev().collect()
        };
        for l in range {
            let neighbors = |v: usize| -> &Vec<usize> { if downward { &preds[v] } else { &succs[v] } };
            let mut keyed: Vec<(f32, usize)> = by_layer[l]
                .iter()
                .map(|&v| {
                    let ns = neighbors(v);
                    let key = if ns.is_empty() {
                        pos[v] as f32
                    } else {
                        ns.iter().map(|&u| pos[u] as f32).sum::<f32>() / ns.len() as f32
                    };
                    (key, v)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            by_layer[l] = keyed.into_iter().map(|(_, v)| v).collect();
        }
    }

    // Primary coordinates: cumulative layer pitch.
    let mut layer_primary = vec![0.0f32; layer_count];
    let mut acc = 0.0f32;
    for l in 0..layer_count {
        let extent = by_layer[l]
            .iter()
            .map(|&v| sizes[v][pi])
            .fold(0.0f32, f32::max);
        layer_primary[l] = acc + extent / 2.0;
        acc += extent + LAYER_GAP;
    }

    // Cross coordinates: barycenter relaxation, then enforce order + min gaps.
    let mut cross = vec![0.0f32; n];
    for row in &by_layer {
        let mut c = 0.0f32;
        for &v in row {
            cross[v] = c + sizes[v][ci] / 2.0;
            c += sizes[v][ci] + NODE_GAP;
        }
    }
    for _ in 0..3 {
        for row in &by_layer {
            for &v in row {
                let ns: Vec<usize> = preds[v].iter().chain(succs[v].iter()).copied().collect();
                if !ns.is_empty() {
                    cross[v] = ns.iter().map(|&u| cross[u]).sum::<f32>() / ns.len() as f32;
                }
            }
            // Restore ordering and minimum gaps with a left-to-right push.
            for i in 1..row.len() {
                let (prev, cur) = (row[i - 1], row[i]);
                let min = cross[prev] + sizes[prev][ci] / 2.0 + NODE_GAP + sizes[cur][ci] / 2.0;
                if cross[cur] < min {
                    cross[cur] = min;
                }
            }
        }
    }

    (0..n)
        .map(|v| {
            let mut p = [0.0f32; 2];
            p[pi] = layer_primary[layers[v]];
            p[ci] = cross[v];
            p
        })
        .collect()
}

/// DFS-based feedback edge removal: back edges are reversed so the layering
/// still pulls the cycle's nodes apart instead of dropping the constraint.
fn break_cycles(n: usize, edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in edges {
        if a < n && b < n && a != b {
            adj[a].push(b);
        }
    }
    #[derive(Clone, Copy, PartialEq)]
    enum State {
        White,
        Gray,
        Black,
    }
    let mut state = vec![State::White; n];
    let mut out = Vec::new();
    // Iterative DFS to survive deep graphs.
    for root in 0..n {
        if state[root] != State::White {
            continue;
        }
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        state[root] = State::Gray;
        while let Some(&mut (v, ref mut i)) = stack.last_mut() {
            if *i < adj[v].len() {
                let w = adj[v][*i];
                *i += 1;
                match state[w] {
                    State::White => {
                        state[w] = State::Gray;
                        stack.push((w, 0));
                        out.push((v, w));
                    }
                    State::Gray => out.push((w, v)), // back edge: reverse it
                    State::Black => out.push((v, w)),
                }
            } else {
                state[v] = State::Black;
                stack.pop();
            }
        }
    }
    out
}

fn assign_layers(n: usize, dag: &[(usize, usize)]) -> Vec<usize> {
    let mut indeg = vec![0usize; n];
    let mut succs: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(a, b) in dag {
        succs[a].push(b);
        indeg[b] += 1;
    }
    let mut layer = vec![0usize; n];
    let mut queue: std::collections::VecDeque<usize> =
        (0..n).filter(|&v| indeg[v] == 0).collect();
    while let Some(v) = queue.pop_front() {
        for &w in &succs[v] {
            if layer[w] < layer[v] + 1 {
                layer[w] = layer[v] + 1;
            }
            indeg[w] -= 1;
            if indeg[w] == 0 {
                queue.push_back(w);
            }
        }
    }
    layer
}
