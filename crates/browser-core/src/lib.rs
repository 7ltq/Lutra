use url::Url;

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
        if value.is_empty() { return Err("Enter a URL or search term"); }
        let candidate = if value.contains("://") {
            value.to_string()
        } else if value == "localhost" || value.starts_with("localhost:") || (value.contains('.') && !value.contains(char::is_whitespace)) {
            format!("https://{value}")
        } else {
            let encoded: String = url::form_urlencoded::byte_serialize(value.as_bytes()).collect();
            format!("https://duckduckgo.com/?q={encoded}")
        };
        let url = Url::parse(&candidate).map_err(|_| "Invalid URL")?;
        Self::validate(&url)?;
        Ok(url)
    }

    fn validate(url: &Url) -> Result<(), &'static str> {
        if !matches!(url.scheme(), "https" | "http") || url.host_str().is_none() {
            return Err("Only HTTP and HTTPS websites are supported");
        }
        if !url.username().is_empty() || url.password().is_some() { return Err("Credentials in URLs are not supported"); }
        Ok(())
    }

    pub fn navigate(&mut self, input: &str) -> Result<Url, &'static str> {
        let url = Self::parse_address(input)?;
        self.commit(url.clone())?;
        Ok(url)
    }

    pub fn commit(&mut self, url: Url) -> Result<(), &'static str> {
        Self::validate(&url)?;
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
}
