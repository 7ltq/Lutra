use url::Url;
use crate::BrowserState;

const MAX_LINKS: usize = 128;
const MAX_CLOSED: usize = 16;

pub fn wheel_lines(x: f32, y: f32, speed: f32) -> (f64, f64) {
    let speed = if speed.is_finite() { speed.clamp(0.5, 3.0) } else { 1.0 };
    (f64::from(x) * 76.0 * f64::from(speed), f64::from(y) * 76.0 * f64::from(speed))
}

#[derive(Default)]
pub struct SessionLinks {
    history: Vec<Url>,
    bookmarks: Vec<Url>,
    closed: Vec<Url>,
    last_visit: Option<Url>,
}

impl SessionLinks {
    pub fn history(&self) -> &[Url] { &self.history }
    pub fn bookmarks(&self) -> &[Url] { &self.bookmarks }

    pub fn record_visit(&mut self, url: Url) {
        if BrowserState::validate_web_url(&url).is_err() || self.last_visit.as_ref() == Some(&url) { return; }
        self.last_visit = Some(url.clone());
        self.history.retain(|entry| entry != &url);
        self.history.push(url);
        if self.history.len() > MAX_LINKS { self.history.remove(0); }
    }

    pub fn toggle_bookmark(&mut self, url: Url) {
        if BrowserState::validate_web_url(&url).is_err() { return; }
        if self.bookmarks.contains(&url) { self.remove_bookmark(&url); }
        else {
            self.bookmarks.push(url);
            if self.bookmarks.len() > MAX_LINKS { self.bookmarks.remove(0); }
        }
    }

    pub fn remove_bookmark(&mut self, url: &Url) { self.bookmarks.retain(|entry| entry != url); }
    pub fn clear_history(&mut self) { self.history.clear(); self.closed.clear(); }

    pub fn record_closed(&mut self, url: Url) {
        if BrowserState::validate_web_url(&url).is_err() { return; }
        self.closed.push(url);
        if self.closed.len() > MAX_CLOSED { self.closed.remove(0); }
    }

    pub fn reopen(&mut self) -> Option<Url> { self.closed.pop() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(index: usize) -> Url { Url::parse(&format!("https://example.com/{index}")).unwrap() }

    #[test]
    fn wheel_scale_preserves_direction_and_fraction() {
        assert_eq!(wheel_lines(0.5, -1.0, 1.0), (38.0, -76.0));
        assert_eq!(wheel_lines(0.0, 1.0, 2.0), (0.0, 152.0));
        assert_eq!(wheel_lines(0.0, 1.0, 10.0), (0.0, 228.0));
    }

    #[test]
    fn history_is_bounded_and_deduplicated() {
        let mut links = SessionLinks::default();
        for i in 0..200 { links.record_visit(url(i)); }
        assert_eq!(links.history().len(), 128);
        links.record_visit(url(199));
        assert_eq!(links.history().len(), 128);
        links.clear_history();
        links.record_visit(url(199));
        assert!(links.history().is_empty());
        links.record_visit(url(200));
        assert_eq!(links.history(), &[url(200)]);
    }

    #[test]
    fn bookmark_toggle_and_reopen_are_session_local() {
        let mut links = SessionLinks::default();
        links.toggle_bookmark(url(1));
        assert_eq!(links.bookmarks(), &[url(1)]);
        links.toggle_bookmark(url(1));
        assert!(links.bookmarks().is_empty());
        links.record_closed(url(1));
        links.record_closed(url(2));
        assert_eq!(links.reopen(), Some(url(2)));
        assert_eq!(links.reopen(), Some(url(1)));
        assert_eq!(links.reopen(), None);
        assert!(SessionLinks::default().bookmarks().is_empty());
    }
}
