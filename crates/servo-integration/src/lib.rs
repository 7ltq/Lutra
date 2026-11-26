use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineState {
    Disconnected,
    Initializing,
    Ready,
    Failed(String),
}

pub struct ServoAdapter {
    state: EngineState,
}

impl Default for ServoAdapter {
    fn default() -> Self { Self { state: EngineState::Disconnected } }
}

impl ServoAdapter {
    pub fn state(&self) -> &EngineState { &self.state }
    pub fn accepts(url: &Url) -> bool { matches!(url.scheme(), "http" | "https") && url.host_str().is_some() && url.username().is_empty() && url.password().is_none() }
}
