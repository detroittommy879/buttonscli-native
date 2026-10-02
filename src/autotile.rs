//! Session-local membership and selection for automatic pane layouts.

use std::collections::BTreeSet;

#[derive(Default)]
pub(crate) struct AutoTile {
    excluded: BTreeSet<u64>,
    pub(crate) requested_count: usize,
}

impl AutoTile {
    pub(crate) fn includes(&self, id: u64) -> bool {
        !self.excluded.contains(&id)
    }

    pub(crate) fn set_included(&mut self, id: u64, included: bool) {
        if included {
            self.excluded.remove(&id);
        } else {
            self.excluded.insert(id);
        }
    }

    pub(crate) fn forget(&mut self, id: u64) -> bool {
        self.excluded.remove(&id)
    }

    pub(crate) fn eligible_count(&self, ids: &[u64]) -> usize {
        ids.iter().filter(|id| self.includes(**id)).count()
    }

    /// Focus first, then preserve the current group, then fill in tab order.
    /// Membership uses IDs so reordering or removing tabs cannot transfer it.
    pub(crate) fn select(&self, ids: &[u64], focused: usize, previous: &[usize]) -> Vec<usize> {
        let count = self.requested_count.clamp(1, 10);
        let mut selected = Vec::with_capacity(count);
        for index in std::iter::once(focused)
            .chain(previous.iter().copied())
            .chain(0..ids.len())
        {
            if selected.len() == count {
                break;
            }
            if ids.get(index).is_some_and(|id| self.includes(*id)) && !selected.contains(&index) {
                selected.push(index);
            }
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_focused_tab_never_leaks_back_into_tiles() {
        let mut tiles = AutoTile {
            requested_count: 4,
            ..Default::default()
        };
        tiles.set_included(20, false);
        assert_eq!(
            tiles.select(&[10, 20, 30, 40, 50], 1, &[1, 3, 3, 99]),
            vec![3, 0, 2, 4]
        );
        tiles.set_included(20, true);
        assert_eq!(
            tiles.select(&[10, 20, 30, 40, 50], 1, &[3, 0, 2, 4]),
            vec![1, 3, 0, 2]
        );
    }

    #[test]
    fn membership_survives_reordering_and_closing_neighbors() {
        let mut tiles = AutoTile {
            requested_count: 10,
            ..Default::default()
        };
        tiles.set_included(20, false);
        assert_eq!(tiles.select(&[30, 10, 20], 2, &[]), vec![0, 1]);
        tiles.forget(10);
        assert_eq!(tiles.select(&[30, 20], 1, &[]), vec![0]);
        assert!(tiles.forget(20));
        assert!(tiles.includes(20));
    }

    #[test]
    fn empty_membership_is_valid_and_does_not_change_requested_count() {
        let mut tiles = AutoTile {
            requested_count: 4,
            ..Default::default()
        };
        tiles.set_included(10, false);
        assert!(tiles.select(&[10], 0, &[0]).is_empty());
        assert_eq!(tiles.requested_count, 4);
        assert_eq!(tiles.select(&[10, 20, 30], 1, &[]), vec![1, 2]);
    }

    #[test]
    fn requested_count_is_bounded_and_all_counts_only_eligible_sessions() {
        let mut tiles = AutoTile {
            requested_count: usize::MAX,
            ..Default::default()
        };
        let ids: Vec<_> = (1..=64).collect();
        tiles.set_included(2, false);
        assert_eq!(tiles.eligible_count(&ids), 63);
        let selected = tiles.select(&ids, 0, &[]);
        assert_eq!(selected.len(), 10);
        assert!(!selected.contains(&1));
    }
}
