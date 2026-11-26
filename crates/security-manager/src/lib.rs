#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission { Ask, Allow, Block }

#[derive(Clone, Debug)]
pub struct PermissionPolicy {
    pub camera: Permission,
    pub microphone: Permission,
    pub location: Permission,
    pub notifications: Permission,
    pub popups: Permission,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self { camera: Permission::Ask, microphone: Permission::Ask, location: Permission::Ask, notifications: Permission::Ask, popups: Permission::Block }
    }
}
