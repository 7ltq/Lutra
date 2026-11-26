use crate::BrowserState;
use url::Url;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchProvider {
    #[default]
    DuckDuckGo,
    Brave,
    Startpage,
}

impl SearchProvider {
    pub const ALL: [Self; 3] = [Self::DuckDuckGo, Self::Brave, Self::Startpage];

    pub fn label(self) -> &'static str {
        match self {
            Self::DuckDuckGo => "DuckDuckGo",
            Self::Brave => "Brave Search",
            Self::Startpage => "Startpage",
        }
    }

    pub fn search(self, query: &str) -> Result<Url, &'static str> {
        let (base, key) = match self {
            Self::DuckDuckGo => ("https://duckduckgo.com/", "q"),
            Self::Brave => ("https://search.brave.com/search", "q"),
            Self::Startpage => ("https://www.startpage.com/sp/search", "query"),
        };
        let mut url = Url::parse(base).map_err(|_| "Invalid search provider")?;
        url.query_pairs_mut().append_pair(key, query);
        Ok(url)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Clone, Debug)]
pub struct Shortcut {
    pub label: String,
    pub url: Url,
}

pub struct BrowserPreferences {
    pub search: SearchProvider,
    pub appearance: Appearance,
    pub max_tabs: usize,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for BrowserPreferences {
    fn default() -> Self {
        let mut settings = Self {
            search: SearchProvider::default(),
            appearance: Appearance::default(),
            max_tabs: 32,
            shortcuts: Vec::new(),
        };
        for (name, url) in [("DuckDuckGo", "https://duckduckgo.com/"), ("Wikipedia", "https://www.wikipedia.org/"), ("GitHub", "https://github.com/")] {
            let _ = settings.add_shortcut(name, url);
        }
        settings
    }
}

impl BrowserPreferences {
    pub fn add_shortcut(&mut self, label: &str, address: &str) -> Result<(), &'static str> {
        if self.shortcuts.len() >= 12 { return Err("Maximum of 12 shortcuts"); }
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 40 || label.chars().any(char::is_control) {
            return Err("Enter a name of 1–40 characters");
        }
        let url = Url::parse(address.trim()).map_err(|_| "Enter a full HTTPS address")?;
        BrowserState::validate_web_url(&url)?;
        if self.shortcuts.iter().any(|shortcut| shortcut.url == url) { return Err("Shortcut already exists"); }
        self.shortcuts.push(Shortcut { label: label.to_owned(), url });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_provider_encodes_query_without_changing_direct_navigation() {
        let u = BrowserState::parse_address_with("a & b", SearchProvider::Brave).unwrap();
        assert_eq!(u.host_str(), Some("search.brave.com"));
        assert_eq!(u.query_pairs().find(|(k, _)| k == "q").unwrap().1, "a & b");
        assert_eq!(BrowserState::parse_address_with("https://example.com/", SearchProvider::Brave).unwrap().host_str(), Some("example.com"));
    }

    #[test]
    fn shortcuts_are_validated_and_bounded() {
        let mut p = BrowserPreferences::default();
        assert!(p.add_shortcut("Unsafe", "file:///secret").is_err());
        assert!(p.add_shortcut("Duplicate", "https://github.com/").is_err());
        assert!(p.add_shortcut("Example", "https://example.com/").is_ok());
        for i in 0..20 { let _ = p.add_shortcut("Link", &format!("https://example.com/{i}")); }
        assert_eq!(p.shortcuts.len(), 12);
    }
}
