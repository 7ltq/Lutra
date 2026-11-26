use url::Url;

pub mod native_tabs;

#[derive(Clone, Debug)]
pub struct Tab {
    pub id: u64,
    pub title: String,
    pub url: Option<Url>,
    history: Vec<Url>,
    cursor: usize,
}

#[derive(Default)]
pub struct BrowserState {
    tabs: Vec<Tab>,
    active: Option<u64>,
    next_id: u64,
}

impl BrowserState {
    pub fn new() -> Self { Self::default() }
    pub fn tabs(&self) -> &[Tab] { &self.tabs }
    pub fn active_id(&self) -> Option<u64> { self.active }
    pub fn active(&self) -> Option<&Tab> { self.tabs.iter().find(|t| Some(t.id) == self.active) }

    pub fn open_tab(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("Tab ID exhausted");
        self.tabs.push(Tab { id, title: "New Tab".into(), url: None, history: Vec::new(), cursor: 0 });
        self.active = Some(id);
        id
    }

    pub fn select_tab(&mut self, id: u64) -> bool {
        if self.tabs.iter().any(|t| t.id == id) { self.active = Some(id); true } else { false }
    }

    pub fn close_tab(&mut self, id: u64) -> bool {
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else { return false };
        self.tabs.remove(index);
        if self.active == Some(id) {
            self.active = self.tabs.get(index.min(self.tabs.len().saturating_sub(1))).map(|t| t.id);
        }
        true
    }

    pub fn parse_address(input: &str) -> Result<Url, &'static str> {
        let value = input.trim();
        if value.is_empty() {
            return Err("Enter a URL or search term");
        }
        if value.len() > 8192 || value.chars().any(char::is_control) {
            return Err("Invalid address");
        }

        let direct = value.starts_with("https://") || value.starts_with("http://");
        let authority = value.split(['/', '?', '#']).next().unwrap_or_default();
        let host_port = authority.rsplit_once(':').is_some_and(|(host, port)| {
            !host.is_empty() && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
                && (host == "localhost" || host.contains('.'))
        });
        let looks_like_host = !value.contains(char::is_whitespace)
            && (authority.contains('.') || authority == "localhost" || host_port);

        let candidate = if direct {
            value.to_owned()
        } else if value.contains("://") || (authority.contains(':') && !host_port) {
            return Err("Only HTTPS websites are supported");
        } else if looks_like_host {
            format!("https://{value}")
        } else {
            let mut search = Url::parse("https://duckduckgo.com/").map_err(|_| "Invalid search URL")?;
            search.query_pairs_mut().append_pair("q", value);
            return Ok(search);
        };

        let url = Url::parse(&candidate).map_err(|_| "Invalid address")?;
        Self::validate_web_url(&url)?;
        Ok(url)
    }

    pub fn validate_web_url(url: &Url) -> Result<(), &'static str> {
        if url.scheme() != "https" || url.host_str().is_none() {
            return Err("Only HTTPS websites are supported");
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err("Credentials in URLs are not supported");
        }
        Ok(())
    }

    pub fn navigate(&mut self, input: &str) -> Result<Url, &'static str> {
        let url = Self::parse_address(input)?;
        self.commit(url.clone())?;
        Ok(url)
    }

    pub fn commit(&mut self, url: Url) -> Result<(), &'static str> {
        Self::validate_web_url(&url)?;
        let tab = self.tabs.iter_mut().find(|t| Some(t.id) == self.active).ok_or("No active tab")?;
        if tab.url.as_ref() != Some(&url) {
            tab.history.truncate(if tab.history.is_empty() { 0 } else { tab.cursor + 1 });
            tab.history.push(url.clone());
            if tab.history.len() > 128 { tab.history.remove(0); }
            tab.cursor = tab.history.len() - 1;
        }
        tab.title = url.host_str().unwrap_or("Website").to_string();
        tab.url = Some(url);
        Ok(())
    }

    pub fn history_target(&self, forward: bool) -> Option<&Url> {
        let tab = self.active()?;
        let index = if forward { tab.cursor.checked_add(1)? } else { tab.cursor.checked_sub(1)? };
        tab.history.get(index)
    }

    pub fn move_history(&mut self, forward: bool) -> bool {
        let Some(url) = self.history_target(forward).cloned() else { return false };
        let Some(tab) = self.tabs.iter_mut().find(|t| Some(t.id) == self.active) else { return false };
        if forward { tab.cursor += 1; } else { tab.cursor -= 1; }
        tab.title = url.host_str().unwrap_or("Website").to_owned();
        tab.url = Some(url);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_branches_after_back() {
        let mut b = BrowserState::new();
        b.open_tab();
        b.navigate("https://example.com").unwrap();
        b.navigate("https://example.org").unwrap();
        assert!(b.move_history(false));
        assert_eq!(b.active().unwrap().url.as_ref().unwrap().host_str(), Some("example.com"));
        b.navigate("https://example.net").unwrap();
        assert!(b.history_target(true).is_none());
    }

    #[test]
    fn tab_management() {
        let mut b = BrowserState::new();
        let a = b.open_tab();
        let c = b.open_tab();
        assert_eq!(b.active_id(), Some(c));
        assert!(b.select_tab(a));
        assert!(b.close_tab(a));
        assert_eq!(b.active_id(), Some(c));
    }

    #[test]
    fn rejects_local_file_schemes() {
        let mut b = BrowserState::new();
        b.open_tab();
        assert!(b.navigate("file:///secret").is_err());
        assert!(b.commit(Url::parse("file:///secret").unwrap()).is_err());
    }

    #[test]
    fn rejects_credentials() {
        let mut b = BrowserState::new();
        b.open_tab();
        assert!(b.navigate("https://user:password@example.com").is_err());
    }

    #[test]
    fn encodes_search_terms() {
        let mut b = BrowserState::new();
        b.open_tab();
        let u = b.navigate("hello world").unwrap();
        assert_eq!(u.host_str(), Some("duckduckgo.com"));
        assert_eq!(u.query_pairs().find(|(k, _)| k == "q").unwrap().1, "hello world");
    }

    #[test]
    fn history_is_bounded() {
        let mut b = BrowserState::new();
        b.open_tab();
        for n in 0..200 { b.navigate(&format!("https://example.com/{n}")).unwrap(); }
        assert_eq!(b.active().unwrap().history.len(), 128);
        for _ in 0..127 { assert!(b.move_history(false)); }
        assert!(!b.move_history(false));
    }
    #[test]
    fn rejects_insecure_and_external_schemes() {
        for input in ["http://example.com", "javascript:alert(1)", "mailto:test@example.com", "file:///private", "https:example.com"] {
            assert!(BrowserState::parse_address(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_credentials_and_control_characters() {
        for input in ["https://user@example.com", "user:secret@example.com", "https://example.com/\npath"] {
            assert!(BrowserState::parse_address(input).is_err(), "{input}");
        }
    }

    #[test]
    fn adds_https_to_bare_hosts() {
        for input in ["example.com", "example.com:8443", "localhost:8443"] {
            assert_eq!(BrowserState::parse_address(input).unwrap().scheme(), "https");
        }
    }
}
