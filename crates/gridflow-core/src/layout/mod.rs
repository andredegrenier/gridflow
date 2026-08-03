//! Layout engine. Auto-placed nodes go through the layered algorithm (cached
//! by a structural hash and seeded from previous positions so typing a label
//! never moves nodes); pinned nodes keep their exact coordinates; relative
//! hints resolve last. Groups lay out as local clusters, then the cluster is
//! placed (pinned groups at their coordinate, auto groups as supernodes in the
//! top-level graph).

pub mod layered;
pub mod relative;

use crate::ast::{Diagnostic, LayoutDir};
use crate::geometry::{node_size, Rect, TextMeasurer};
use crate::model::{DiagramModel, NodeId, Placement};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

pub const GROUP_PADDING: f32 = 20.0;
pub const GROUP_TITLE_H: f32 = 24.0;

#[derive(Debug, Clone, Default)]
pub struct LayoutResult {
    /// World-space node centers.
    pub positions: HashMap<NodeId, [f32; 2]>,
    pub sizes: HashMap<NodeId, [f32; 2]>,
    pub group_rects: HashMap<NodeId, Rect>,
    /// Layout-time problems (placement cycles, unknown anchors).
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
pub struct LayoutEngine {
    prev: Option<(u64, LayoutResult)>,
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compute(&mut self, model: &DiagramModel, measurer: &dyn TextMeasurer) -> LayoutResult {
        let mut result = LayoutResult::default();

        for node in model.nodes.values() {
            result
                .sizes
                .insert(node.id.clone(), node_size(node, measurer));
        }

        let hash = structural_hash(model, &result.sizes);
        let reuse = self
            .prev
            .as_ref()
            .filter(|(h, _)| *h == hash)
            .map(|(_, r)| r.positions.clone());

        match reuse {
            Some(prev_positions) => {
                result.positions = prev_positions;
                // Drop stale entries for removed nodes.
                let ids: std::collections::HashSet<&NodeId> = model.nodes.keys().collect();
                result.positions.retain(|id, _| ids.contains(id));
            }
            None => {
                let seed: HashMap<NodeId, [f32; 2]> = self
                    .prev
                    .as_ref()
                    .map(|(_, r)| r.positions.clone())
                    .unwrap_or_default();
                result.positions = full_layout(model, &result.sizes, &seed);
            }
        }

        // Pinned coordinates always win, and are applied fresh each pass so a
        // drag doesn't invalidate the cached auto layout.
        for node in model.nodes.values() {
            if let Placement::Absolute { pos, .. } = node.placement {
                result.positions.insert(node.id.clone(), pos);
            }
        }
        for group in model.groups.values() {
            if let Placement::Absolute { pos, .. } = group.placement {
                translate_group_to(model, &mut result, &group.id, pos);
            }
        }

        result.diagnostics =
            relative::resolve(model, &mut result.positions, &result.sizes);

        // Relative-placed groups anchor off a node like nodes do.
        for group in model.groups.values() {
            if let Placement::Relative { anchor, dir, gap, .. } = &group.placement {
                if let Some(rect) = group_rect(model, &result, &group.id) {
                    if let (Some(&apos), Some(&asize)) = (
                        result.positions.get(anchor),
                        result.sizes.get(anchor),
                    ) {
                        let gsize = rect.size();
                        let gap = gap.unwrap_or(relative::DEFAULT_GAP);
                        let center = match dir {
                            crate::ast::Dir::RightOf => {
                                [apos[0] + asize[0] / 2.0 + gap + gsize[0] / 2.0, apos[1]]
                            }
                            crate::ast::Dir::LeftOf => {
                                [apos[0] - asize[0] / 2.0 - gap - gsize[0] / 2.0, apos[1]]
                            }
                            crate::ast::Dir::Above => {
                                [apos[0], apos[1] - asize[1] / 2.0 - gap - gsize[1] / 2.0]
                            }
                            crate::ast::Dir::Below => {
                                [apos[0], apos[1] + asize[1] / 2.0 + gap + gsize[1] / 2.0]
                            }
                        };
                        translate_group_to(model, &mut result, &group.id, center);
                    }
                }
            }
        }

        for group in model.groups.values() {
            if let Some(rect) = group_rect(model, &result, &group.id) {
                result.group_rects.insert(group.id.clone(), rect);
            }
        }

        self.prev = Some((hash, result.clone()));
        result
    }
}

/// Bounding rect of a group's members (with padding and title strip).
fn group_rect(model: &DiagramModel, result: &LayoutResult, gid: &NodeId) -> Option<Rect> {
    let group = model.groups.get(gid)?;
    let mut rect = Rect::NOTHING;
    for m in &group.members {
        if let (Some(&pos), Some(&size)) = (result.positions.get(m), result.sizes.get(m)) {
            rect = rect.union(Rect::from_center_size(pos, size));
        }
    }
    if !rect.is_finite() {
        return None;
    }
    let mut r = rect.expand(GROUP_PADDING);
    r.min[1] -= GROUP_TITLE_H;
    Some(r)
}

fn translate_group_to(model: &DiagramModel, result: &mut LayoutResult, gid: &NodeId, center: [f32; 2]) {
    let Some(rect) = group_rect(model, result, gid) else { return };
    let c = rect.center();
    let delta = [center[0] - c[0], center[1] - c[1]];
    let Some(group) = model.groups.get(gid) else { return };
    for m in &group.members {
        // A member pinned with its own `@` stays exactly where the user put it.
        let pinned = matches!(
            model.nodes.get(m).map(|n| &n.placement),
            Some(Placement::Absolute { .. })
        );
        if pinned {
            continue;
        }
        if let Some(p) = result.positions.get_mut(m) {
            p[0] += delta[0];
            p[1] += delta[1];
        }
    }
}

/// Full layered layout: groups first (locally), then top level with auto
/// groups as supernodes.
fn full_layout(
    model: &DiagramModel,
    sizes: &HashMap<NodeId, [f32; 2]>,
    seed: &HashMap<NodeId, [f32; 2]>,
) -> HashMap<NodeId, [f32; 2]> {
    let mut positions: HashMap<NodeId, [f32; 2]> = HashMap::new();

    // 1. Each group's members in local coordinates.
    let mut group_sizes: HashMap<NodeId, [f32; 2]> = HashMap::new();
    for group in model.groups.values() {
        let dir = group.dir.unwrap_or(model.direction);
        let local = scope_layout(model, sizes, seed, &group.members, dir);
        let mut rect = Rect::NOTHING;
        for (id, pos) in &local {
            rect = rect.union(Rect::from_center_size(*pos, sizes[id]));
        }
        positions.extend(local);
        if rect.is_finite() {
            let r = rect.expand(GROUP_PADDING);
            group_sizes.insert(
                group.id.clone(),
                [r.max[0] - r.min[0], r.max[1] - r.min[1] + GROUP_TITLE_H],
            );
        }
    }

    // 2. Top level: ungrouped nodes plus one supernode per auto-placed group.
    #[derive(Clone, PartialEq, Eq, Hash)]
    enum Item {
        Node(NodeId),
        Group(NodeId),
    }
    let member_of: HashMap<&NodeId, &NodeId> = model
        .groups
        .values()
        .flat_map(|g| g.members.iter().map(move |m| (m, &g.id)))
        .collect();

    let mut items: Vec<Item> = Vec::new();
    for node in model.nodes.values() {
        if node.group.is_none() {
            items.push(Item::Node(node.id.clone()));
        }
    }
    for group in model.groups.values() {
        // Pinned/relative groups are placed later; only auto groups join the
        // top-level layered graph.
        if matches!(group.placement, Placement::Auto) {
            items.push(Item::Group(group.id.clone()));
        }
    }

    let index: HashMap<Item, usize> = items
        .iter()
        .enumerate()
        .map(|(i, it)| (it.clone(), i))
        .collect();
    let lift = |id: &NodeId| -> Option<usize> {
        if let Some(gid) = member_of.get(id) {
            index.get(&Item::Group((*gid).clone())).copied()
        } else {
            index.get(&Item::Node(id.clone())).copied()
        }
    };

    let mut edges = Vec::new();
    for e in &model.edges {
        if let (Some(a), Some(b)) = (lift(&e.from), lift(&e.to)) {
            if a != b {
                edges.push((a, b));
            }
        }
    }

    let item_sizes: Vec<[f32; 2]> = items
        .iter()
        .map(|it| match it {
            Item::Node(id) => sizes.get(id).copied().unwrap_or([80.0, 40.0]),
            Item::Group(id) => group_sizes.get(id).copied().unwrap_or([120.0, 80.0]),
        })
        .collect();
    let item_seed = |i: usize| -> Option<[f32; 2]> {
        match &items[i] {
            Item::Node(id) => seed.get(id).copied(),
            Item::Group(_) => None,
        }
    };
    let laid = layered::layout(&item_sizes, &edges, model.direction, item_seed);

    for (i, item) in items.iter().enumerate() {
        match item {
            Item::Node(id) => {
                positions.insert(id.clone(), laid[i]);
            }
            Item::Group(gid) => {
                // Translate the group's local cluster to its supernode slot.
                let group = &model.groups[gid];
                let mut rect = Rect::NOTHING;
                for m in &group.members {
                    if let (Some(&pos), Some(&size)) = (positions.get(m), sizes.get(m)) {
                        rect = rect.union(Rect::from_center_size(pos, size));
                    }
                }
                if rect.is_finite() {
                    let c = rect.center();
                    let delta = [laid[i][0] - c[0], laid[i][1] - c[1]];
                    for m in &group.members {
                        if let Some(p) = positions.get_mut(m) {
                            p[0] += delta[0];
                            p[1] += delta[1];
                        }
                    }
                }
            }
        }
    }
    positions
}

fn scope_layout(
    model: &DiagramModel,
    sizes: &HashMap<NodeId, [f32; 2]>,
    seed: &HashMap<NodeId, [f32; 2]>,
    members: &[NodeId],
    dir: LayoutDir,
) -> HashMap<NodeId, [f32; 2]> {
    let index: HashMap<&NodeId, usize> =
        members.iter().enumerate().map(|(i, id)| (id, i)).collect();
    let member_sizes: Vec<[f32; 2]> = members
        .iter()
        .map(|id| sizes.get(id).copied().unwrap_or([80.0, 40.0]))
        .collect();
    let mut edges = Vec::new();
    for e in &model.edges {
        if let (Some(&a), Some(&b)) = (index.get(&e.from), index.get(&e.to)) {
            if a != b {
                edges.push((a, b));
            }
        }
    }
    let laid = layered::layout(&member_sizes, &edges, dir, |i| {
        seed.get(&members[i]).copied()
    });
    members
        .iter()
        .zip(laid)
        .map(|(id, pos)| (id.clone(), pos))
        .collect()
}

/// Hash of everything the *auto* layout depends on. Absolute coordinates are
/// deliberately excluded: dragging a pinned node must not relayout the rest.
fn structural_hash(model: &DiagramModel, sizes: &HashMap<NodeId, [f32; 2]>) -> u64 {
    let mut h = DefaultHasher::new();
    model.direction.hash(&mut h);
    for node in model.nodes.values() {
        node.id.hash(&mut h);
        node.shape.hash(&mut h);
        node.group.hash(&mut h);
        if let Some(size) = sizes.get(&node.id) {
            ((size[0] * 4.0) as i64).hash(&mut h);
            ((size[1] * 4.0) as i64).hash(&mut h);
        }
        match &node.placement {
            Placement::Auto => 0u8.hash(&mut h),
            Placement::Absolute { .. } => 1u8.hash(&mut h),
            Placement::Relative { anchor, dir, gap, .. } => {
                2u8.hash(&mut h);
                anchor.hash(&mut h);
                dir.hash(&mut h);
                gap.map(|g| (g * 4.0) as i64).hash(&mut h);
            }
        }
    }
    for group in model.groups.values() {
        group.id.hash(&mut h);
        group.dir.hash(&mut h);
        matches!(group.placement, Placement::Auto).hash(&mut h);
        group.members.hash(&mut h);
    }
    for e in &model.edges {
        e.from.hash(&mut h);
        e.to.hash(&mut h);
    }
    h.finish()
}
