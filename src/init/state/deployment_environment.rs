#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DeploymentEnvironment {
    Local,
    Dev,
    Staging,
    Prod,
}
