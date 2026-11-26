use url::Url;

const STARTER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "google-analytics.com",
    "googlesyndication.com",
    "adnxs.com",
];

#[derive(Default)]
pub struct NetworkPolicy {
    pub block_known_domains: bool,
    custom: Vec<String>,
    blocked: u64,
}

impl NetworkPolicy {
    pub fn blocked_count(&self) -> u64 { self.blocked }
    pub fn custom_domains(&self) -> &[String] { &self.custom }

    pub fn add_domain(&mut self, input: &str) -> Result<(), &'static str> {
        let domain = input.trim().trim_end_matches('.').to_ascii_lowercase();
        if domain.len() > 253 || !domain.contains('.') || domain.split('.').any(|label| label.is_empty() || label.len() > 63 || label.starts_with('-') || label.ends_with('-') || !label.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')) {
            return Err("Enter a hostname such as tracker.example.com");
        }
        if self.custom.len() >= 256 { return Err("Domain limit reached"); }
        if !self.custom.contains(&domain) { self.custom.push(domain); }
        Ok(())
    }

    pub fn remove_domain(&mut self, index: usize) {
        if index < self.custom.len() { self.custom.remove(index); }
    }

    fn matches_domain(host: &str, domain: &str) -> bool {
        host == domain || host.strip_suffix(domain).is_some_and(|prefix| prefix.ends_with('.'))
    }

    pub fn should_block(&mut self, url: &Url) -> bool {
        let blocked = url.scheme() == "http" || url.host_str().is_some_and(|host| {
            let host = host.trim_end_matches('.');
            self.custom.iter().any(|domain| Self::matches_domain(host, domain))
                || (self.block_known_domains && STARTER_DOMAINS.iter().any(|domain| Self::matches_domain(host, domain)))
        });
        if blocked { self.blocked = self.blocked.saturating_add(1); }
        blocked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_match_boundaries_and_subdomains() {
        let mut p = NetworkPolicy::default();
        p.add_domain("Tracker.Example.com.").unwrap();
        for address in ["https://tracker.example.com/", "https://sub.tracker.example.com/"] {
            assert!(p.should_block(&Url::parse(address).unwrap()));
        }
        assert!(!p.should_block(&Url::parse("https://nottracker.example.com/").unwrap()));
        assert!(!p.should_block(&Url::parse("https://tracker.example.com.evil.test/").unwrap()));
        assert_eq!(p.blocked_count(), 2);
    }

    #[test]
    fn http_is_blocked_and_builtin_list_is_opt_in() {
        let mut p = NetworkPolicy::default();
        assert!(p.should_block(&Url::parse("http://example.com/").unwrap()));
        let u = Url::parse("https://a.doubleclick.net/").unwrap();
        assert!(!p.should_block(&u));
        p.block_known_domains = true;
        assert!(p.should_block(&u));
        assert!(p.add_domain("https://example.com/path").is_err());
    }
}
