pub const MAX_TABS: usize = 32;

pub struct NativeTabs<T> {
    slots: Vec<Option<T>>,
    active: usize,
    pinned: Vec<bool>,
}

impl<T> Default for NativeTabs<T> {
    fn default() -> Self {
        Self { slots: vec![None], active: 0, pinned: vec![false] }
    }
}

impl<T> NativeTabs<T> {
    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn slots(&self) -> &[Option<T>] {
        &self.slots
    }

    pub fn active(&self) -> Option<&T> {
        self.slots[self.active].as_ref()
    }

    pub fn insert_active(&mut self, value: T) -> Result<(), T> {
        let slot = &mut self.slots[self.active];
        if slot.is_some() {
            return Err(value);
        }
        *slot = Some(value);
        Ok(())
    }

    pub fn open(&mut self) -> bool {
        if self.slots.len() >= MAX_TABS {
            return false;
        }
        self.slots.push(None);
        self.pinned.push(false);
        self.active = self.slots.len() - 1;
        true
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.slots.len() || index == self.active {
            return false;
        }
        self.active = index;
        true
    }

    pub fn is_pinned(&self, index: usize) -> bool {
        self.pinned.get(index).copied().unwrap_or(false)
    }

    pub fn toggle_pin(&mut self, index: usize) -> bool {
        let Some(pin) = self.pinned.get_mut(index) else { return false };
        *pin = !*pin;
        true
    }

    pub fn move_tab(&mut self, from: usize, to: usize) -> bool {
        if from >= self.slots.len() || to >= self.slots.len() || from == to { return false; }
        let slot = self.slots.remove(from);
        let pin = self.pinned.remove(from);
        self.slots.insert(to, slot);
        self.pinned.insert(to, pin);
        self.active = if self.active == from { to }
            else if from < self.active && self.active <= to { self.active - 1 }
            else if to <= self.active && self.active < from { self.active + 1 }
            else { self.active };
        true
    }

    pub fn close(&mut self, index: usize) -> bool {
        if index >= self.slots.len() {
            return false;
        }
        self.slots.remove(index);
        self.pinned.remove(index);
        if self.slots.is_empty() {
            self.slots.push(None);
            self.pinned.push(false);
            self.active = 0;
        } else if index < self.active {
            self.active -= 1;
        } else if index == self.active {
            self.active = index.min(self.slots.len() - 1);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn moving_preserves_selection_and_pin() {
        let mut tabs = NativeTabs::default();
        tabs.insert_active(1).unwrap();
        tabs.toggle_pin(0);
        tabs.open();
        tabs.insert_active(2).unwrap();
        assert!(tabs.move_tab(0, 1));
        assert_eq!(tabs.active(), Some(&2));
        assert!(tabs.is_pinned(1));
        assert!(!tabs.is_pinned(0));
        tabs.close(1);
        assert_eq!(tabs.slots().len(), 1);
        assert!(!tabs.is_pinned(0));
    }

    #[test]
    fn new_tabs_are_lazy() {
        let mut tabs = NativeTabs::<u32>::default();
        assert!(tabs.active().is_none());
        tabs.insert_active(42).unwrap();
        assert!(tabs.open());
        assert!(tabs.active().is_none());
        assert!(tabs.select(0));
        assert_eq!(tabs.active(), Some(&42));
    }

    #[test]
    fn closing_before_active_preserves_active_value() {
        let mut tabs = NativeTabs::default();
        tabs.insert_active(1).unwrap();
        tabs.open();
        tabs.insert_active(2).unwrap();
        tabs.open();
        tabs.insert_active(3).unwrap();
        assert!(tabs.close(0));
        assert_eq!(tabs.active_index(), 1);
        assert_eq!(tabs.active(), Some(&3));
    }

    #[test]
    fn closing_last_tab_creates_empty_slot() {
        let mut tabs = NativeTabs::default();
        tabs.insert_active(7).unwrap();
        assert!(tabs.close(0));
        assert_eq!(tabs.slots().len(), 1);
        assert_eq!(tabs.active_index(), 0);
        assert!(tabs.active().is_none());
    }

    #[test]
    fn rejects_invalid_selection_and_replacement() {
        let mut tabs = NativeTabs::default();
        tabs.insert_active(1).unwrap();
        assert_eq!(tabs.insert_active(2), Err(2));
        assert!(!tabs.select(9));
        assert!(!tabs.close(9));
        assert_eq!(tabs.active(), Some(&1));
    }

    #[test]
    fn bounded_tab_count() {
        let mut tabs = NativeTabs::<u32>::default();
        for _ in 1..MAX_TABS {
            assert!(tabs.open());
        }
        assert!(!tabs.open());
        assert_eq!(tabs.slots().len(), MAX_TABS);
    }

    #[test]
    fn closing_drops_only_the_removed_view() {
        struct DropCounter(Rc<Cell<usize>>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }
        let count = Rc::new(Cell::new(0));
        let mut tabs = NativeTabs::default();
        assert!(tabs.insert_active(DropCounter(count.clone())).is_ok());
        tabs.open();
        assert!(tabs.insert_active(DropCounter(count.clone())).is_ok());
        assert!(tabs.close(0));
        assert_eq!(count.get(), 1);
        drop(tabs);
        assert_eq!(count.get(), 2);
    }
}
