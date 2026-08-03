//! Resolves `right-of` / `left-of` / `above` / `below` placement hints.
//! Hints may chain (`c right-of b`, `b right-of a`), so dependents are
//! processed in topological order; cycles fall back to the auto position
//! with a diagnostic.

use crate::ast::{Diagnostic, Dir};
use crate::model::{DiagramModel, NodeId, Placement};
use std::collections::HashMap;

pub const DEFAULT_GAP: f32 = 50.0;

/// Overwrites `positions` for every relative-placed node. `sizes` must contain
/// every node. Returns diagnostics for cycles/unknown anchors.
pub fn resolve(
    model: &DiagramModel,
    positions: &mut HashMap<NodeId, [f32; 2]>,
    sizes: &HashMap<NodeId, [f32; 2]>,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();

    let rel: Vec<(&NodeId, &NodeId, Dir, f32)> = model
        .nodes
        .values()
        .filter_map(|n| match &n.placement {
            Placement::Relative { anchor, dir, gap, .. } => {
                Some((&n.id, anchor, *dir, gap.unwrap_or(DEFAULT_GAP)))
            }
            _ => None,
        })
        .collect();

    // Kahn's algorithm over anchor -> dependent links (only links where the
    // anchor is itself relative-placed constrain the order).
    let rel_ids: std::collections::HashSet<&NodeId> = rel.iter().map(|r| r.0).collect();
    let mut indeg: HashMap<&NodeId, usize> = rel.iter().map(|r| (r.0, 0)).collect();
    for (id, anchor, _, _) in &rel {
        if rel_ids.contains(anchor) {
            *indeg.get_mut(id).unwrap() += 1;
        }
    }
    let mut queue: Vec<&NodeId> = indeg
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&id, _)| id)
        .collect();
    let mut done = 0usize;
    while let Some(id) = queue.pop() {
        done += 1;
        let (_, anchor, dir, gap) = *rel.iter().find(|r| r.0 == id).unwrap();
        place(model, positions, sizes, id, anchor, dir, gap, &mut diags);
        for (dep, dep_anchor, _, _) in &rel {
            if *dep_anchor == id {
                let d = indeg.get_mut(dep).unwrap();
                *d -= 1;
                if *d == 0 {
                    queue.push(dep);
                }
            }
        }
    }
    if done < rel.len() {
        for (id, _, _, _) in &rel {
            if indeg[*id] > 0 {
                if let Some(node) = model.nodes.get(*id) {
                    diags.push(Diagnostic::error(
                        node.stmt_span,
                        format!("placement of `{id}` is part of a cycle; using auto layout"),
                    ));
                }
            }
        }
    }
    diags
}

#[allow(clippy::too_many_arguments)]
fn place(
    model: &DiagramModel,
    positions: &mut HashMap<NodeId, [f32; 2]>,
    sizes: &HashMap<NodeId, [f32; 2]>,
    id: &NodeId,
    anchor: &NodeId,
    dir: Dir,
    gap: f32,
    diags: &mut Vec<Diagnostic>,
) {
    let (Some(&apos), Some(&asize)) = (positions.get(anchor), sizes.get(anchor)) else {
        if let Some(node) = model.nodes.get(id) {
            diags.push(Diagnostic::error(
                node.stmt_span,
                format!("unknown anchor `{anchor}` for placement of `{id}`"),
            ));
        }
        return;
    };
    let osize = sizes.get(id).copied().unwrap_or([80.0, 40.0]);
    let pos = match dir {
        Dir::RightOf => [apos[0] + asize[0] / 2.0 + gap + osize[0] / 2.0, apos[1]],
        Dir::LeftOf => [apos[0] - asize[0] / 2.0 - gap - osize[0] / 2.0, apos[1]],
        Dir::Above => [apos[0], apos[1] - asize[1] / 2.0 - gap - osize[1] / 2.0],
        Dir::Below => [apos[0], apos[1] + asize[1] / 2.0 + gap + osize[1] / 2.0],
    };
    positions.insert(id.clone(), pos);
}
