//! The terminal area's split layout (feature 484): a binary tree of panes, each showing at most
//! one terminal, with one focused pane.
//!
//! Render-free by construction (Constitution I): every decision (refuse a split, where focus goes,
//! which terminal a pane shows, how the area is tiled) lives here and is unit-tested; the client's
//! `SplitView` widget only draws what [`PaneLayout::place`] returns.
//!
//! Invariants, enforced by the operations so a violation is unrepresentable: 1 ≤ leaves ≤
//! [`MAX_PANES`]; `focused` names a leaf; a [`TerminalRef`] appears in at most one leaf; `PaneId`s
//! are unique; every ratio lies in [`RATIO_MIN`], [`RATIO_MAX`].

pub use crate::protocol::messages::{SessionProcess, TerminalRef};

/// The most panes one project may show at once (FR-001).
pub const MAX_PANES: usize = 6;
/// Smallest pane, in terminal columns (research R10).
pub const MIN_PANE_COLS: u16 = 20;
/// Smallest pane, in terminal rows, excluding the header strip (research R10).
pub const MIN_PANE_ROWS: u16 = 5;
/// Lowest divider ratio.
pub const RATIO_MIN: f32 = 0.05;
/// Highest divider ratio.
pub const RATIO_MAX: f32 = 0.95;

/// Stable identity of a pane within its layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PaneId(u32);

impl PaneId {
    /// The raw number (persistence).
    pub fn get(self) -> u32 {
        self.0
    }
}

/// How a split divides its area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Children side by side (a vertical divider).
    Vertical,
    /// Children stacked (a horizontal divider).
    Horizontal,
}

/// A direction to move focus in (feature 484, FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Towards the left edge.
    Left,
    /// Towards the right edge.
    Right,
    /// Towards the top edge.
    Up,
    /// Towards the bottom edge.
    Down,
}

/// One pane: a terminal, or empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    id: PaneId,
    terminal: Option<TerminalRef>,
}

impl Pane {
    /// The pane's identity.
    pub fn id(&self) -> PaneId {
        self.id
    }
    /// What it shows; `None` is the empty-pane state.
    pub fn terminal(&self) -> Option<TerminalRef> {
        self.terminal
    }
}

/// A node of the layout tree.
#[derive(Debug, Clone, PartialEq)]
pub enum PaneNode {
    /// A pane.
    Leaf(Pane),
    /// Two children divided along `axis`; `ratio` is the first child's share.
    Split {
        /// The direction children sit in.
        axis: Axis,
        /// First child's share of the extent, in [`RATIO_MIN`]..=[`RATIO_MAX`].
        ratio: f32,
        /// Left / top child.
        first: Box<PaneNode>,
        /// Right / bottom child.
        second: Box<PaneNode>,
    },
}

/// Why an operation was refused. The client shows [`Refusal::reason`] next to the pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Already [`MAX_PANES`] panes.
    TooManyPanes,
    /// The pane is too small to halve.
    TooSmall,
    /// No such pane.
    UnknownPane,
    /// The last pane cannot be closed (the area always has one).
    LastPane,
}

impl Refusal {
    /// A sentence for the user.
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::TooManyPanes => "At most 6 panes fit.",
            Refusal::TooSmall => "This pane is too small to split.",
            Refusal::UnknownPane => "That pane is gone.",
            Refusal::LastPane => "The last pane stays open.",
        }
    }
}

/// A rectangle in the units the caller passes to [`PaneLayout::place`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub w: f32,
    /// Height.
    pub h: f32,
}

/// The line between two siblings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Divider {
    /// The whole area of the split this line divides (its two children together).
    pub area: Rect,
    /// `Vertical`: a vertical line between side-by-side children.
    pub axis: Axis,
    /// Where the line is; `w` or `h` is zero along the axis, the other spans the split's extent.
    pub rect: Rect,
}

/// Where everything goes for one area.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Each pane's rectangle, in tree order.
    pub panes: Vec<(PaneId, Rect)>,
    /// Each split's divider.
    pub dividers: Vec<Divider>,
}

/// The split layout of one project's terminal area.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneLayout {
    root: PaneNode,
    focused: PaneId,
    next_id: u32,
}

impl Default for PaneLayout {
    fn default() -> Self {
        Self::single()
    }
}

fn leaves_of<'a>(node: &'a PaneNode, out: &mut Vec<&'a Pane>) {
    match node {
        PaneNode::Leaf(p) => out.push(p),
        PaneNode::Split { first, second, .. } => {
            leaves_of(first, out);
            leaves_of(second, out);
        }
    }
}

fn find_leaf_mut(node: &mut PaneNode, id: PaneId) -> Option<&mut PaneNode> {
    match node {
        PaneNode::Leaf(p) if p.id == id => Some(node),
        PaneNode::Leaf(_) => None,
        PaneNode::Split { first, second, .. } => {
            find_leaf_mut(first, id).or_else(move || find_leaf_mut(second, id))
        }
    }
}

/// The `n`th split in tree (pre-)order, counting down.
fn split_at_mut<'a>(node: &'a mut PaneNode, n: &mut usize) -> Option<&'a mut PaneNode> {
    if matches!(node, PaneNode::Leaf(_)) {
        return None;
    }
    if *n == 0 {
        return Some(node);
    }
    *n -= 1;
    let PaneNode::Split { first, second, .. } = node else {
        return None;
    };
    split_at_mut(first, n).or_else(move || split_at_mut(second, n))
}

/// Panes `node` shows side by side along `axis`: what an equal share is measured in.
fn panes_along(node: &PaneNode, axis: Axis) -> usize {
    match node {
        PaneNode::Leaf(_) => 1,
        PaneNode::Split {
            axis: a,
            first,
            second,
            ..
        } => {
            let (f, s) = (panes_along(first, axis), panes_along(second, axis));
            if *a == axis {
                f + s
            } else {
                f.max(s)
            }
        }
    }
}

/// The surviving leaf nearest a closed one: the sibling's edge that touched it.
fn nearest_leaf(node: &PaneNode, from_first_side: bool) -> PaneId {
    match node {
        PaneNode::Leaf(p) => p.id,
        PaneNode::Split { first, second, .. } => nearest_leaf(
            if from_first_side { first } else { second },
            from_first_side,
        ),
    }
}

/// Remove leaf `id` from `node`: its sibling takes the parent's place. Returns the leaf focus
/// should move to when `id` was found.
fn close_in(node: &mut PaneNode, id: PaneId) -> Option<PaneId> {
    let PaneNode::Split { first, second, .. } = node else {
        return None;
    };
    let closed_first = matches!(&**first, PaneNode::Leaf(p) if p.id == id);
    let closed_second = matches!(&**second, PaneNode::Leaf(p) if p.id == id);
    if closed_first || closed_second {
        let keep = if closed_first { second } else { first };
        let keep = std::mem::replace(&mut **keep, PaneNode::Leaf(Pane { id, terminal: None }));
        let near = nearest_leaf(&keep, closed_first);
        *node = keep;
        return Some(near);
    }
    close_in(first, id).or_else(|| close_in(second, id))
}

/// Smallest extent `node` needs along `axis`.
fn min_extent(node: &PaneNode, axis: Axis, min: (f32, f32)) -> f32 {
    let m = match axis {
        Axis::Vertical => min.0,
        Axis::Horizontal => min.1,
    };
    match node {
        PaneNode::Leaf(_) => m,
        PaneNode::Split {
            axis: a,
            first,
            second,
            ..
        } => {
            let (f, s) = (min_extent(first, axis, min), min_extent(second, axis, min));
            if *a == axis {
                f + s
            } else {
                f.max(s)
            }
        }
    }
}

fn place_node(node: &PaneNode, r: Rect, min: (f32, f32), out: &mut Placement) {
    match node {
        PaneNode::Leaf(p) => out.panes.push((p.id, r)),
        PaneNode::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let extent = match axis {
                Axis::Vertical => r.w,
                Axis::Horizontal => r.h,
            };
            let (fm, sm) = (
                min_extent(first, *axis, min),
                min_extent(second, *axis, min),
            );
            let want = extent * ratio;
            let f = if extent >= fm + sm {
                want.clamp(fm, extent - sm)
            } else if fm + sm > 0.0 {
                extent * fm / (fm + sm)
            } else {
                want
            };
            let (ra, rb, line) = match axis {
                Axis::Vertical => (
                    Rect { w: f, ..r },
                    Rect {
                        x: r.x + f,
                        w: extent - f,
                        ..r
                    },
                    Rect {
                        x: r.x + f,
                        w: 0.0,
                        ..r
                    },
                ),
                Axis::Horizontal => (
                    Rect { h: f, ..r },
                    Rect {
                        y: r.y + f,
                        h: extent - f,
                        ..r
                    },
                    Rect {
                        y: r.y + f,
                        h: 0.0,
                        ..r
                    },
                ),
            };
            out.dividers.push(Divider {
                area: r,
                axis: *axis,
                rect: line,
            });
            place_node(first, ra, min, out);
            place_node(second, rb, min, out);
        }
    }
}

impl PaneLayout {
    /// One empty pane, focused: today's single-terminal area.
    pub fn single() -> Self {
        let id = PaneId(1);
        Self {
            root: PaneNode::Leaf(Pane { id, terminal: None }),
            focused: id,
            next_id: 2,
        }
    }

    /// The tree.
    pub fn root(&self) -> &PaneNode {
        &self.root
    }

    /// The focused pane.
    pub fn focused(&self) -> PaneId {
        self.focused
    }

    /// Every pane, in tree order.
    pub fn panes(&self) -> Vec<&Pane> {
        let mut v = Vec::new();
        leaves_of(&self.root, &mut v);
        v
    }

    /// Number of panes.
    pub fn len(&self) -> usize {
        self.panes().len()
    }

    /// Never true: a layout always has a pane.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// One pane by id.
    pub fn pane(&self, id: PaneId) -> Option<&Pane> {
        self.panes().into_iter().find(|p| p.id == id)
    }

    /// The pane showing `t`, if any.
    pub fn pane_showing(&self, t: TerminalRef) -> Option<PaneId> {
        self.panes()
            .into_iter()
            .find(|p| p.terminal == Some(t))
            .map(|p| p.id)
    }

    /// The terminal the focused pane shows.
    pub fn focused_terminal(&self) -> Option<TerminalRef> {
        self.pane(self.focused).and_then(|p| p.terminal)
    }

    /// Every terminal shown, in tree order.
    pub fn terminals(&self) -> Vec<TerminalRef> {
        self.panes().iter().filter_map(|p| p.terminal).collect()
    }

    /// Focus `id`; false when there is no such pane.
    pub fn focus(&mut self, id: PaneId) -> bool {
        if self.pane(id).is_some() {
            self.focused = id;
            true
        } else {
            false
        }
    }

    /// Move focus to the nearest pane in `dir` (FR-009): judged on the unit-square tiling, the
    /// neighbour whose edge is closest and whose extent overlaps the focused pane's most. Stays
    /// put when there is none. True when focus moved.
    pub fn focus_dir(&mut self, dir: Direction) -> bool {
        const EPS: f32 = 1e-4;
        let rects = self.rects((1.0, 1.0), (0.0, 0.0));
        let Some((_, cur)) = rects.iter().find(|(id, _)| *id == self.focused).copied() else {
            return false;
        };
        let overlap = |a0: f32, a1: f32, b0: f32, b1: f32| (a1.min(b1) - a0.max(b0)).max(0.0);
        let mut best: Option<(PaneId, f32, f32)> = None;
        for (id, r) in rects {
            if id == self.focused {
                continue;
            }
            let (gap, shared) = match dir {
                Direction::Left => (
                    cur.x - (r.x + r.w),
                    overlap(cur.y, cur.y + cur.h, r.y, r.y + r.h),
                ),
                Direction::Right => (
                    r.x - (cur.x + cur.w),
                    overlap(cur.y, cur.y + cur.h, r.y, r.y + r.h),
                ),
                Direction::Up => (
                    cur.y - (r.y + r.h),
                    overlap(cur.x, cur.x + cur.w, r.x, r.x + r.w),
                ),
                Direction::Down => (
                    r.y - (cur.y + cur.h),
                    overlap(cur.x, cur.x + cur.w, r.x, r.x + r.w),
                ),
            };
            if gap < -EPS || shared <= EPS {
                continue;
            }
            let better = best.is_none_or(|(_, g, s)| {
                gap < g - EPS || ((gap - g).abs() <= EPS && shared > s + EPS)
            });
            if better {
                best = Some((id, gap, shared));
            }
        }
        match best {
            Some((id, _, _)) => self.focus(id),
            None => false,
        }
    }

    /// Split `pane` along `axis`; the new pane comes second, takes focus and shows `candidate`
    /// unless that terminal is already shown (FR-004: never a duplicate). `pane_size` is the
    /// pane's current size and `min` the smallest pane, in the same units.
    pub fn split(
        &mut self,
        pane: PaneId,
        axis: Axis,
        pane_size: (f32, f32),
        min: (f32, f32),
        candidate: Option<TerminalRef>,
    ) -> Result<PaneId, Refusal> {
        if self.pane(pane).is_none() {
            return Err(Refusal::UnknownPane);
        }
        if self.len() >= MAX_PANES {
            return Err(Refusal::TooManyPanes);
        }
        let (have, need) = match axis {
            Axis::Vertical => (pane_size.0, min.0),
            Axis::Horizontal => (pane_size.1, min.1),
        };
        if have < 2.0 * need {
            return Err(Refusal::TooSmall);
        }
        let new_id = PaneId(self.next_id);
        let terminal = candidate.filter(|t| self.pane_showing(*t).is_none());
        let node = find_leaf_mut(&mut self.root, pane).ok_or(Refusal::UnknownPane)?;
        let old = std::mem::replace(
            node,
            PaneNode::Leaf(Pane {
                id: new_id,
                terminal: None,
            }),
        );
        *node = PaneNode::Split {
            axis,
            ratio: 0.5,
            first: Box::new(old),
            second: Box::new(PaneNode::Leaf(Pane {
                id: new_id,
                terminal,
            })),
        };
        self.next_id += 1;
        self.focused = new_id;
        Ok(new_id)
    }

    /// Show `t` in `pane` and focus it. A terminal already shown in another pane is never
    /// duplicated: that pane takes focus instead and nothing else changes.
    pub fn show(&mut self, pane: PaneId, t: TerminalRef) -> Result<(), Refusal> {
        if let Some(at) = self.pane_showing(t) {
            self.focused = at;
            return Ok(());
        }
        match find_leaf_mut(&mut self.root, pane) {
            Some(PaneNode::Leaf(p)) => p.terminal = Some(t),
            _ => return Err(Refusal::UnknownPane),
        }
        self.focused = pane;
        Ok(())
    }

    /// Show `t` in the focused pane, or focus the pane already showing it (tab strip, sidebar).
    pub fn show_or_focus(&mut self, t: TerminalRef) {
        let at = self.focused;
        let _ = self.show(at, t);
    }

    /// Close `pane`: its sibling takes the freed space (FR-006). The terminal it showed is only
    /// no longer shown; nothing here touches the session. When the focused pane closes, focus
    /// moves to the nearest surviving pane. The last pane is refused.
    pub fn close(&mut self, pane: PaneId) -> Result<(), Refusal> {
        if self.pane(pane).is_none() {
            return Err(Refusal::UnknownPane);
        }
        if self.len() < 2 {
            return Err(Refusal::LastPane);
        }
        let near = close_in(&mut self.root, pane).ok_or(Refusal::UnknownPane)?;
        if self.focused == pane {
            self.focused = near;
        }
        Ok(())
    }

    /// Swap the terminals of `a` and `b` (FR-008) and focus `b`, where the moved terminal now is.
    pub fn swap(&mut self, a: PaneId, b: PaneId) -> Result<(), Refusal> {
        let (Some(ta), Some(tb)) = (
            self.pane(a).map(Pane::terminal),
            self.pane(b).map(Pane::terminal),
        ) else {
            return Err(Refusal::UnknownPane);
        };
        if let Some(PaneNode::Leaf(p)) = find_leaf_mut(&mut self.root, a) {
            p.terminal = tb;
        }
        if let Some(PaneNode::Leaf(p)) = find_leaf_mut(&mut self.root, b) {
            p.terminal = ta;
        }
        self.focused = b;
        Ok(())
    }

    /// Set the `index`th divider (the order of [`Placement::dividers`]) to `ratio` of its split,
    /// kept within both children's minimum sizes for an area of `total`. False when there is no
    /// such divider.
    pub fn set_ratio(
        &mut self,
        index: usize,
        ratio: f32,
        total: (f32, f32),
        min: (f32, f32),
    ) -> bool {
        let area = self.place(total, min).dividers.get(index).map(|d| d.area);
        let mut n = index;
        let Some(PaneNode::Split {
            axis,
            ratio: r,
            first,
            second,
        }) = split_at_mut(&mut self.root, &mut n)
        else {
            return false;
        };
        let Some(area) = area else {
            return false;
        };
        let extent = match axis {
            Axis::Vertical => area.w,
            Axis::Horizontal => area.h,
        };
        let (fm, sm) = (
            min_extent(first, *axis, min),
            min_extent(second, *axis, min),
        );
        let mut lo = RATIO_MIN;
        let mut hi = RATIO_MAX;
        if extent > 0.0 && extent >= fm + sm {
            lo = lo.max(fm / extent);
            hi = hi.min(1.0 - sm / extent);
        }
        *r = ratio.clamp(lo, hi.max(lo));
        true
    }

    /// The `index`th divider's share for its first child.
    pub fn ratio(&self, index: usize) -> Option<f32> {
        fn nth(n: &PaneNode, i: &mut usize) -> Option<f32> {
            let PaneNode::Split {
                ratio,
                first,
                second,
                ..
            } = n
            else {
                return None;
            };
            if *i == 0 {
                return Some(*ratio);
            }
            *i -= 1;
            nth(first, i).or_else(|| nth(second, i))
        }
        nth(&self.root, &mut { index })
    }

    /// Give the `index`th divider's two sides equal pane shares (FR-005, the double press).
    pub fn reset_equal(&mut self, index: usize) -> bool {
        let mut n = index;
        let Some(PaneNode::Split {
            axis,
            ratio,
            first,
            second,
        }) = split_at_mut(&mut self.root, &mut n)
        else {
            return false;
        };
        let (f, s) = (panes_along(first, *axis), panes_along(second, *axis));
        *ratio = f as f32 / (f + s) as f32;
        true
    }

    /// Empty every pane whose terminal `live` rejects. Never removes a pane.
    pub fn prune(&mut self, live: impl Fn(TerminalRef) -> bool) {
        fn walk(n: &mut PaneNode, live: &dyn Fn(TerminalRef) -> bool) {
            match n {
                PaneNode::Leaf(p) => {
                    if p.terminal.is_some_and(|t| !live(t)) {
                        p.terminal = None;
                    }
                }
                PaneNode::Split { first, second, .. } => {
                    walk(first, live);
                    walk(second, live);
                }
            }
        }
        walk(&mut self.root, &live);
    }

    /// Tile `total` (width, height). Ratios are honoured within the minimums; when `total` is
    /// below the layout's total minimum, shares scale down in proportion to their minimums so the
    /// panes still tile the whole area without overlap.
    pub fn place(&self, total: (f32, f32), min: (f32, f32)) -> Placement {
        let mut out = Placement {
            panes: Vec::new(),
            dividers: Vec::new(),
        };
        let r = Rect {
            x: 0.0,
            y: 0.0,
            w: total.0,
            h: total.1,
        };
        place_node(&self.root, r, min, &mut out);
        out
    }

    /// Each pane's rectangle ([`Self::place`] without the dividers).
    pub fn rects(&self, total: (f32, f32), min: (f32, f32)) -> Vec<(PaneId, Rect)> {
        self.place(total, min).panes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{SessionId, ShellInstanceId};

    const MIN: (f32, f32) = (100.0, 50.0);
    const BIG: (f32, f32) = (1000.0, 1000.0);

    fn t(n: u128, shell: Option<u32>) -> TerminalRef {
        TerminalRef {
            session: SessionId::from_uuid(uuid::Uuid::from_u128(n)),
            process: shell.map_or(SessionProcess::Primary, |s| {
                SessionProcess::Shell(ShellInstanceId(s))
            }),
        }
    }

    fn invariants(l: &PaneLayout) {
        let panes = l.panes();
        assert!((1..=MAX_PANES).contains(&panes.len()));
        assert!(panes.iter().any(|p| p.id == l.focused()));
        let mut ids: Vec<_> = panes.iter().map(|p| p.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), panes.len(), "PaneIds unique");
        let shown = l.terminals();
        let mut d = shown.clone();
        d.dedup();
        for (i, a) in shown.iter().enumerate() {
            assert!(!shown[i + 1..].contains(a), "duplicate terminal");
        }
        drop(d);
    }

    fn grid() -> (PaneLayout, PaneId, PaneId, PaneId) {
        // [a | b over c]
        let mut l = PaneLayout::single();
        let a = l.focused();
        let b = l.split(a, Axis::Vertical, BIG, MIN, None).unwrap();
        let c = l.split(b, Axis::Horizontal, BIG, MIN, None).unwrap();
        (l, a, b, c)
    }

    #[test]
    fn focus_dir_moves_to_the_nearest_neighbour_and_stays_at_the_edge() {
        let (mut l, a, b, c) = grid();
        assert_eq!(l.focused(), c);
        assert!(l.focus_dir(Direction::Up));
        assert_eq!(l.focused(), b);
        assert!(!l.focus_dir(Direction::Up), "no pane above");
        assert_eq!(l.focused(), b);
        assert!(l.focus_dir(Direction::Left));
        assert_eq!(l.focused(), a);
        assert!(!l.focus_dir(Direction::Left));
        assert!(!l.focus_dir(Direction::Up));
        assert!(!l.focus_dir(Direction::Down));
        assert!(l.focus_dir(Direction::Right));
        assert_eq!(
            l.focused(),
            b,
            "the neighbour with the largest overlap, first in tree order"
        );
        assert!(l.focus_dir(Direction::Down));
        assert_eq!(l.focused(), c);
        assert!(!l.focus_dir(Direction::Down));
        invariants(&l);
    }

    #[test]
    fn focus_dir_on_a_single_pane_stays_put() {
        let mut l = PaneLayout::single();
        for d in [
            Direction::Left,
            Direction::Right,
            Direction::Up,
            Direction::Down,
        ] {
            assert!(!l.focus_dir(d));
        }
    }

    #[test]
    fn a_new_layout_is_one_focused_empty_pane() {
        let l = PaneLayout::single();
        assert_eq!(l.len(), 1);
        assert_eq!(l.focused_terminal(), None);
        invariants(&l);
    }

    #[test]
    fn split_adds_a_pane_after_and_focuses_it() {
        let mut l = PaneLayout::single();
        let first = l.focused();
        l.show(first, t(1, None)).unwrap();
        let new = l
            .split(first, Axis::Vertical, BIG, MIN, Some(t(2, None)))
            .unwrap();
        assert_eq!(l.focused(), new);
        assert_eq!(l.terminals(), vec![t(1, None), t(2, None)]);
        invariants(&l);
    }

    #[test]
    fn a_candidate_already_shown_gives_an_empty_pane() {
        let mut l = PaneLayout::single();
        let first = l.focused();
        l.show(first, t(1, None)).unwrap();
        let new = l
            .split(first, Axis::Horizontal, BIG, MIN, Some(t(1, None)))
            .unwrap();
        assert_eq!(l.pane(new).unwrap().terminal(), None);
        assert_eq!(l.terminals(), vec![t(1, None)]);
    }

    #[test]
    fn the_seventh_split_is_refused() {
        let mut l = PaneLayout::single();
        for _ in 1..MAX_PANES {
            let f = l.focused();
            l.split(f, Axis::Vertical, BIG, MIN, None).unwrap();
        }
        let f = l.focused();
        assert_eq!(
            l.split(f, Axis::Vertical, BIG, MIN, None),
            Err(Refusal::TooManyPanes)
        );
        assert_eq!(l.len(), 6);
        assert!(!Refusal::TooManyPanes.reason().is_empty());
    }

    #[test]
    fn a_pane_below_twice_the_minimum_is_not_split_along_that_axis() {
        let mut l = PaneLayout::single();
        let f = l.focused();
        assert_eq!(
            l.split(f, Axis::Vertical, (150.0, 1000.0), MIN, None),
            Err(Refusal::TooSmall)
        );
        assert!(l
            .split(f, Axis::Horizontal, (150.0, 1000.0), MIN, None)
            .is_ok());
    }

    #[test]
    fn an_unknown_pane_is_refused() {
        let mut l = PaneLayout::single();
        assert_eq!(
            l.split(PaneId(99), Axis::Vertical, BIG, MIN, None),
            Err(Refusal::UnknownPane)
        );
        assert_eq!(l.show(PaneId(99), t(1, None)), Err(Refusal::UnknownPane));
        assert!(!l.focus(PaneId(99)));
    }

    #[test]
    fn showing_a_terminal_shown_elsewhere_focuses_that_pane() {
        let mut l = PaneLayout::single();
        let a = l.focused();
        l.show(a, t(1, None)).unwrap();
        let b = l.split(a, Axis::Vertical, BIG, MIN, None).unwrap();
        l.show(b, t(2, None)).unwrap();
        l.show(b, t(1, None)).unwrap();
        assert_eq!(l.focused(), a);
        assert_eq!(l.pane(b).unwrap().terminal(), Some(t(2, None)));
        l.show_or_focus(t(2, None));
        assert_eq!(l.focused(), b);
        l.show_or_focus(t(3, Some(1)));
        assert_eq!(l.pane(b).unwrap().terminal(), Some(t(3, Some(1))));
        invariants(&l);
    }

    #[test]
    fn prune_empties_gone_terminals_and_keeps_every_pane() {
        let mut l = PaneLayout::single();
        let a = l.focused();
        l.show(a, t(1, None)).unwrap();
        let b = l
            .split(a, Axis::Vertical, BIG, MIN, Some(t(2, None)))
            .unwrap();
        l.prune(|x| x != t(2, None));
        assert_eq!(l.len(), 2);
        assert_eq!(l.pane(a).unwrap().terminal(), Some(t(1, None)));
        assert_eq!(l.pane(b).unwrap().terminal(), None);
    }

    fn tiles(l: &PaneLayout, total: (f32, f32)) {
        let p = l.place(total, MIN);
        let area: f32 = p.panes.iter().map(|(_, r)| r.w * r.h).sum();
        assert!(
            (area - total.0 * total.1).abs() < 0.5,
            "area {area} vs {}",
            total.0 * total.1
        );
        for (i, (_, a)) in p.panes.iter().enumerate() {
            assert!(a.x >= -0.01 && a.y >= -0.01);
            assert!(a.x + a.w <= total.0 + 0.01 && a.y + a.h <= total.1 + 0.01);
            for (_, b) in &p.panes[i + 1..] {
                let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
                let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
                assert!(ox <= 0.01 || oy <= 0.01, "overlap {a:?} {b:?}");
            }
        }
    }

    #[test]
    fn rects_tile_the_area_and_scale_down_below_the_total_minimum() {
        let mut l = PaneLayout::single();
        let mut seed = 7u32;
        while l.len() < 4 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let at = l.panes()[(seed >> 8) as usize % l.len()].id();
            let axis = if seed & 1 == 0 {
                Axis::Vertical
            } else {
                Axis::Horizontal
            };
            let _ = l.split(at, axis, BIG, MIN, None);
        }
        for total in [(1000.0, 800.0), (400.0, 300.0), (50.0, 40.0)] {
            tiles(&l, total);
        }
        let p = l.place((1000.0, 800.0), MIN);
        assert!(p.panes.iter().all(|(_, r)| r.w >= MIN.0 && r.h >= MIN.1));
        assert_eq!(p.dividers.len(), 3);
    }

    #[test]
    fn random_operation_sequences_keep_the_invariants() {
        let mut l = PaneLayout::single();
        let mut seed = 12345u32;
        for step in 0..400u32 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let r = (seed >> 8) as usize;
            let pane = l.panes()[r % l.len()].id();
            match r % 5 {
                0 | 1 => {
                    let axis = if r & 8 == 0 {
                        Axis::Vertical
                    } else {
                        Axis::Horizontal
                    };
                    let _ = l.split(pane, axis, BIG, MIN, Some(t((r % 9) as u128, None)));
                }
                2 => {
                    let _ = l.show(pane, t((r % 9) as u128, Some((r % 2) as u32)));
                }
                3 => l.show_or_focus(t((r % 9) as u128, None)),
                _ => l.prune(|x| x.session.0.as_u128() % 3 != u128::from(step % 3)),
            }
            invariants(&l);
            tiles(&l, (900.0, 700.0));
        }
    }

    #[test]
    fn closing_a_pane_gives_its_space_to_the_sibling_and_moves_focus_to_the_nearest_survivor() {
        let (mut l, a, b, c) = grid();
        assert_eq!(l.focused(), c);
        l.close(c).unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l.focused(), b, "the sibling that touched the closed pane");
        let rects = l.rects(BIG, MIN);
        assert_eq!(rects[1].1.h, BIG.1, "b fills the freed height");
        l.close(a).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l.focused(), b, "focus unchanged when another pane closed");
        assert_eq!(l.close(b), Err(Refusal::LastPane));
        assert_eq!(l.close(PaneId(99)), Err(Refusal::UnknownPane));
        invariants(&l);
    }

    #[test]
    fn closing_keeps_the_other_terminals_and_unfocused_focus() {
        let mut l = PaneLayout::single();
        let a = l.focused();
        l.show(a, t(1, None)).unwrap();
        let b = l
            .split(a, Axis::Vertical, BIG, MIN, Some(t(2, None)))
            .unwrap();
        let c = l
            .split(b, Axis::Vertical, BIG, MIN, Some(t(3, None)))
            .unwrap();
        l.focus(a);
        l.close(b).unwrap();
        assert_eq!(l.focused(), a);
        assert_eq!(l.terminals(), vec![t(1, None), t(3, None)]);
        assert!(l.pane(c).is_some());
        l.focus(c);
        l.close(c).unwrap();
        assert_eq!(l.focused(), a, "nearest leaf of the sibling");
    }

    #[test]
    fn swap_exchanges_terminals_including_an_empty_pane() {
        let mut l = PaneLayout::single();
        let a = l.focused();
        l.show(a, t(1, None)).unwrap();
        let b = l.split(a, Axis::Vertical, BIG, MIN, None).unwrap();
        l.swap(a, b).unwrap();
        assert_eq!(l.pane(a).unwrap().terminal(), None);
        assert_eq!(l.pane(b).unwrap().terminal(), Some(t(1, None)));
        assert_eq!(l.focused(), b);
        assert_eq!(l.swap(a, PaneId(99)), Err(Refusal::UnknownPane));
        invariants(&l);
    }

    #[test]
    fn set_ratio_is_clamped_by_the_minimum_sizes() {
        let (mut l, ..) = grid();
        // Divider 0 is the vertical one between a and (b over c): 1000 wide, min 100.
        assert!(l.set_ratio(0, 0.3, BIG, MIN));
        assert_eq!(l.place(BIG, MIN).panes[0].1.w, 300.0);
        assert!(l.set_ratio(0, 0.0, BIG, MIN));
        assert_eq!(
            l.place(BIG, MIN).panes[0].1.w,
            100.0,
            "never below the minimum"
        );
        assert!(l.set_ratio(0, 1.0, BIG, MIN));
        assert_eq!(l.place(BIG, MIN).panes[1].1.w, 100.0);
        // Divider 1 is the horizontal one: 1000 high, min 50.
        assert!(l.set_ratio(1, 0.99, BIG, MIN));
        assert_eq!(l.place(BIG, MIN).panes[2].1.h, 50.0);
        assert!(!l.set_ratio(2, 0.5, BIG, MIN), "no third divider");
        invariants(&l);
    }

    #[test]
    fn reset_equal_gives_every_pane_along_the_axis_the_same_share() {
        let mut l = PaneLayout::single();
        let a = l.focused();
        let b = l.split(a, Axis::Vertical, BIG, MIN, None).unwrap();
        l.split(b, Axis::Vertical, BIG, MIN, None).unwrap();
        l.set_ratio(0, 0.7, BIG, MIN);
        assert!(l.reset_equal(0));
        let w: Vec<f32> = l.place(BIG, MIN).panes.iter().map(|(_, r)| r.w).collect();
        assert!(w.iter().all(|x| (x - 1000.0 / 3.0).abs() < 0.01), "{w:?}");
        assert!(!l.reset_equal(5));
    }
}
