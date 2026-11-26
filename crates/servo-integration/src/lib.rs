use browser_core::BrowserState;
use url::Url;

pub struct ServoAdapter;

impl ServoAdapter {
    pub fn accepts(url: &Url) -> bool {
        BrowserState::validate_web_url(url).is_ok()
    }
}
