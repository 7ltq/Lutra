use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabActivity {
    Active,
    Background,
    SuspensionCandidate,
    Suspended,
}

#[derive(Debug)]
struct TabResource {
    activity: TabActivity,
    last_active: Instant,
    audio_playing: bool,
    downloading: bool,
}

pub struct ResourceManager {
    tabs: HashMap<u64, TabResource>,
    suspension_delay: Duration,
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self { tabs: HashMap::new(), suspension_delay: Duration::from_secs(300) }
    }
}

impl ResourceManager {
    pub fn new(suspension_delay: Duration) -> Self {
        Self { suspension_delay, ..Self::default() }
    }

    pub fn register(&mut self, id: u64) {
        self.tabs.entry(id).or_insert_with(|| TabResource {
            activity: TabActivity::Background,
            last_active: Instant::now(),
            audio_playing: false,
            downloading: false,
        });
    }

    pub fn remove(&mut self, id: u64) { self.tabs.remove(&id); }

    pub fn activate(&mut self, id: u64) {
        if !self.tabs.contains_key(&id) { return; }
        let now = Instant::now();
        for (tab_id, tab) in &mut self.tabs {
            if *tab_id == id {
                tab.activity = TabActivity::Active;
                tab.last_active = now;
            } else if tab.activity == TabActivity::Active {
                tab.activity = TabActivity::Background;
                tab.last_active = now;
            }
        }
    }

    pub fn protect(&mut self, id: u64, audio: bool, download: bool) {
        if let Some(tab) = self.tabs.get_mut(&id) {
            tab.audio_playing = audio;
            tab.downloading = download;
            if (audio || download) && tab.activity == TabActivity::SuspensionCandidate {
                tab.activity = TabActivity::Background;
            }
        }
    }

    pub fn suspension_candidates(&mut self, now: Instant) -> Vec<u64> {
        let mut result = Vec::new();
        for (id, tab) in &mut self.tabs {
            if tab.activity == TabActivity::Background
                && !tab.audio_playing && !tab.downloading
                && now.saturating_duration_since(tab.last_active) >= self.suspension_delay
            {
                tab.activity = TabActivity::SuspensionCandidate;
                result.push(*id);
            }
        }
        result
    }

    pub fn mark_suspended(&mut self, id: u64) -> bool {
        if let Some(tab) = self.tabs.get_mut(&id) {
            if tab.activity == TabActivity::SuspensionCandidate && !tab.audio_playing && !tab.downloading {
                tab.activity = TabActivity::Suspended;
                return true;
            }
        }
        false
    }

    pub fn activity(&self, id: u64) -> Option<TabActivity> {
        self.tabs.get(&id).map(|tab| tab.activity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_tabs_are_candidates() {
        let mut manager = ResourceManager::new(Duration::ZERO);
        manager.register(1);
        manager.register(2);
        manager.activate(1);
        assert_eq!(manager.suspension_candidates(Instant::now()), vec![2]);
        assert_eq!(manager.activity(1), Some(TabActivity::Active));
    }

    #[test]
    fn audio_and_downloads_prevent_suspension() {
        let mut manager = ResourceManager::new(Duration::ZERO);
        manager.register(1);
        manager.register(2);
        manager.protect(1, true, false);
        manager.protect(2, false, true);
        assert!(manager.suspension_candidates(Instant::now()).is_empty());
    }

    #[test]
    fn protection_revokes_candidate() {
        let mut manager = ResourceManager::new(Duration::ZERO);
        manager.register(1);
        assert_eq!(manager.suspension_candidates(Instant::now()), vec![1]);
        manager.protect(1, true, false);
        assert!(!manager.mark_suspended(1));
    }

    #[test]
    fn unknown_activation_does_not_change_active_tab() {
        let mut manager = ResourceManager::default();
        manager.register(1);
        manager.activate(1);
        manager.activate(99);
        assert_eq!(manager.activity(1), Some(TabActivity::Active));
    }
}
