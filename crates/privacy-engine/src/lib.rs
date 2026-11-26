#[derive(Clone, Debug)]
pub struct PrivacyPolicy {
    pub block_trackers: bool,
    pub partition_storage: bool,
    pub reduce_fingerprinting: bool,
    pub global_privacy_control: bool,
}

impl Default for PrivacyPolicy {
    fn default() -> Self {
        Self { block_trackers: true, partition_storage: true, reduce_fingerprinting: true, global_privacy_control: true }
    }
}
