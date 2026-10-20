//! Where an expanded Overview card is drawn (feature 1121, research R6, contract K3).
//!
//! Pure: no I/O and no logging (FR-016, FR-018). The card widget only draws at the place computed
//! here, so the rule is tested without the GUI.

/// Where an expanded card is drawn, in the grid's coordinates (data-model *Placement*).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// Its top edge.
    pub top: f32,
    /// Its height: its natural height, capped at the view's.
    pub height: f32,
    /// Its content is taller than the view and scrolls inside the card (FR-008).
    pub scrolls: bool,
}

/// Where a card whose grid slot is `slot` (`(top, height)`) and whose expanded content needs
/// `natural` is drawn inside a view `view` (`(top, height)`) (K3, FR-008): as tall as its content,
/// up to the view's height; at its slot's top, unless that would end below the view, then moved up
/// just enough to end at the view's bottom, but never above the view's top nor below its slot's top.
pub fn place(slot: (f32, f32), natural: f32, view: (f32, f32)) -> Placement {
    let (slot_top, _) = slot;
    let (view_top, view_height) = view;
    let height = natural.min(view_height);
    let view_bottom = view_top + view_height;
    let top = if slot_top + height <= view_bottom {
        slot_top
    } else {
        (view_bottom - height).max(view_top).min(slot_top)
    };
    Placement {
        top,
        height,
        scrolls: natural > view_height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The view in every test: 600 px tall, scrolled 1000 px down.
    const VIEW: (f32, f32) = (1000.0, 600.0);
    const VIEW_BOTTOM: f32 = VIEW.0 + VIEW.1;
    const SLOT_HEIGHT: f32 = 200.0;

    /// K3: a card that fits below its slot's top stays at its slot, as tall as its content.
    #[test]
    fn a_card_that_fits_stays_at_its_slot() {
        let p = place((1100.0, SLOT_HEIGHT), 320.0, VIEW);
        assert_eq!(
            p,
            Placement {
                top: 1100.0,
                height: 320.0,
                scrolls: false
            },
            "a card that ends inside the view keeps its slot's top"
        );
    }

    /// K3 (US2 sc.5): a card that would end below the view moves up just enough to end at its
    /// bottom.
    #[test]
    fn a_card_that_would_end_below_the_view_moves_up_to_its_bottom() {
        let p = place((1350.0, SLOT_HEIGHT), 400.0, VIEW);
        assert_eq!(
            p.top,
            VIEW_BOTTOM - 400.0,
            "moved up just enough to be seen whole"
        );
        assert_eq!(p.height, 400.0);
        assert!(!p.scrolls);
    }

    /// K3 (US2 sc.6): a card taller than the view is as tall as the view, starts at the view's
    /// top, and scrolls inside.
    #[test]
    fn a_card_taller_than_the_view_is_capped_and_scrolls() {
        let p = place((1200.0, SLOT_HEIGHT), 900.0, VIEW);
        assert_eq!(
            p,
            Placement {
                top: VIEW.0,
                height: VIEW.1,
                scrolls: true
            },
            "capped at the view's height, moved to its top, scrolling inside"
        );
    }

    /// K3: the shift never moves a card below its slot's top, even when the slot's top lies above
    /// the view (the Overview scrolled past it).
    #[test]
    fn a_card_is_never_moved_below_its_slots_top() {
        let p = place((900.0, SLOT_HEIGHT), 900.0, VIEW);
        assert_eq!(p.top, 900.0, "a slot above the view keeps its top");
        assert_eq!(p.height, VIEW.1);
    }

    /// K3: whenever the expanded content is at least the slot's height, the drawn card covers its
    /// whole slot — for every slot that ends inside the view (one that ends below it cannot be
    /// both covered and seen whole; the shift keeps the card in view).
    #[test]
    fn the_placed_card_always_covers_its_slot() {
        for slot_top in (700..=(VIEW_BOTTOM - SLOT_HEIGHT) as i32).step_by(25) {
            for natural in (200..1000).step_by(50) {
                let slot = (slot_top as f32, SLOT_HEIGHT);
                let p = place(slot, natural as f32, VIEW);
                assert!(
                    p.top <= slot.0 && p.top + p.height >= slot.0 + slot.1,
                    "slot {slot:?}, natural {natural}: {p:?} must cover the slot"
                );
                assert_eq!(p.height, (natural as f32).min(VIEW.1));
            }
        }
    }
}
