// interpreter/capabilities.rs — Capability-based security flags

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    pub fs_read: bool,
    pub fs_write: bool,
    pub net_client: bool,
    pub net_server: bool,
    pub sys_exec: bool,
    pub env_read: bool,
}

impl Capabilities {
    pub fn all() -> Self {
        Capabilities {
            fs_read: true,
            fs_write: true,
            net_client: true,
            net_server: true,
            sys_exec: true,
            env_read: true,
        }
    }

    pub fn none() -> Self {
        Capabilities {
            fs_read: false,
            fs_write: false,
            net_client: false,
            net_server: false,
            sys_exec: false,
            env_read: false,
        }
    }

    pub fn check_fs_read(&self) -> Result<(), String> {
        if self.fs_read { Ok(()) } else { Err("SecurityError: Filesystem read access denied".to_string()) }
    }

    pub fn check_fs_write(&self) -> Result<(), String> {
        if self.fs_write { Ok(()) } else { Err("SecurityError: Filesystem write access denied".to_string()) }
    }

    pub fn check_net_client(&self) -> Result<(), String> {
        if self.net_client { Ok(()) } else { Err("SecurityError: Network client access denied".to_string()) }
    }

    pub fn check_net_server(&self) -> Result<(), String> {
        if self.net_server { Ok(()) } else { Err("SecurityError: Network server access denied".to_string()) }
    }

    pub fn check_sys_exec(&self) -> Result<(), String> {
        if self.sys_exec { Ok(()) } else { Err("SecurityError: System execution access denied".to_string()) }
    }

    pub fn check_env_read(&self) -> Result<(), String> {
        if self.env_read { Ok(()) } else { Err("SecurityError: Environment variable access denied".to_string()) }
    }
}
