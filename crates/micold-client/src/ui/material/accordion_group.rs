//! `AccordionGroup` — which one section of a set is open (feature 1091, FR-001–FR-003).
//!
//! View state for a caller that wants its [`AccordionSection`](super::AccordionSection)s
//! exclusive: opening one closes the other, pressing the open one closes it. Render-free; a
//! section not placed in a group keeps its own `open` flag. Two groups share nothing.

/// What the user last chose, relative to the caller's default key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Choice<K> {
    /// Nothing pressed since the last reset: the default key is open.
    #[default]
    Default,
    /// The user closed the open section: none is open.
    Closed,
    /// The user opened this key.
    Open(K),
}

/// At most one open key among a set of sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccordionGroup<K> {
    choice: Choice<K>,
}

impl<K> Default for AccordionGroup<K> {
    fn default() -> Self {
        Self {
            choice: Choice::Default,
        }
    }
}

impl<K: Copy + Eq> AccordionGroup<K> {
    /// A group with nothing pressed: the default key, if any, is open.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forget every press: the default key is open again.
    pub fn reset(&mut self) {
        self.choice = Choice::Default;
    }

    /// The open key: the user's, or `default` while nothing was pressed.
    pub fn open_key(&self, default: Option<K>) -> Option<K> {
        match self.choice {
            Choice::Default => default,
            Choice::Closed => None,
            Choice::Open(key) => Some(key),
        }
    }

    /// Whether `key` is the open one.
    pub fn is_open(&self, key: K, default: Option<K>) -> bool {
        self.open_key(default) == Some(key)
    }

    /// Press `key`'s header: close it when open, else open it and close the rest.
    pub fn toggle(&mut self, key: K, default: Option<K>) {
        self.choice = if self.is_open(key, default) {
            Choice::Closed
        } else {
            Choice::Open(key)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// FR-001: the default is open until something is pressed; opening replaces it.
    #[test]
    fn the_default_is_open_until_another_opens() {
        let mut g = AccordionGroup::new();
        assert!(g.is_open(1, Some(1)));
        assert!(!g.is_open(2, Some(1)));
        g.toggle(2, Some(1));
        assert_eq!(g.open_key(Some(1)), Some(2));
        assert!(!g.is_open(1, Some(1)));
    }

    /// FR-002: pressing the open key closes to none, and a reopen works.
    #[test]
    fn pressing_the_open_key_closes_it_and_it_can_reopen() {
        let mut g = AccordionGroup::new();
        g.toggle(1, Some(1));
        assert_eq!(g.open_key(Some(1)), None);
        g.toggle(2, Some(1));
        assert_eq!(g.open_key(Some(1)), Some(2));
        g.toggle(2, Some(1));
        g.toggle(2, Some(1));
        assert_eq!(g.open_key(Some(1)), Some(2));
        let mut none = AccordionGroup::new();
        assert_eq!(none.open_key(None), None);
        none.toggle(5, None);
        assert_eq!(none.open_key(None), Some(5));
    }

    /// FR-007: reset returns to the default.
    #[test]
    fn reset_returns_to_the_default() {
        let mut g = AccordionGroup::new();
        g.toggle(2, Some(1));
        g.reset();
        assert_eq!(g.open_key(Some(1)), Some(1));
        g.toggle(1, Some(1));
        g.reset();
        assert_eq!(g.open_key(Some(1)), Some(1));
    }

    /// FR-003: two groups share nothing.
    #[test]
    fn two_groups_are_independent() {
        let mut a = AccordionGroup::new();
        let b = AccordionGroup::new();
        a.toggle(2, Some(1));
        assert_eq!(b.open_key(Some(1)), Some(1));
        assert_eq!(a.open_key(Some(1)), Some(2));
    }
}
