#![allow(dead_code, unused_imports)]
// target/mod.rs — Target abstraction layer
//
// A `Target` describes the platform for which native code is generated.
// Currently only `windows_x86_64_msvc` is implemented, but the abstraction
// is designed to accommodate Linux x86_64, macOS ARM64, and Windows ARM64
// without hard-coded triples in the compiler stages.

pub mod windows_x64;
pub mod diagnostics;

/// Operating system families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    MacOs,
}

/// CPU architectures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

/// ABI / environment (following LLVM triple convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Env {
    Msvc,
    Gnu,
    Musl,
    None,
}

/// Complete target description used by the LLVM IRGen and toolchain layers.
#[derive(Debug, Clone)]
pub struct Target {
    /// LLVM target triple string, e.g. `"x86_64-pc-windows-msvc"`.
    pub triple:      String,
    /// LLVM data layout string for the target.
    pub data_layout: String,
    pub os:          Os,
    pub arch:        Arch,
    pub env:         Env,
}

impl Target {
    /// The default target for the current host platform.
    pub fn host_default() -> Self {
        #[cfg(target_os = "windows")]
        return windows_x64::windows_x86_64_msvc();

        #[cfg(target_os = "linux")]
        return Self {
            triple:      "x86_64-unknown-linux-gnu".to_string(),
            data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128".to_string(),
            os:   Os::Linux,
            arch: Arch::X86_64,
            env:  Env::Gnu,
        };

        #[cfg(target_os = "macos")]
        return Self {
            triple:      "arm64-apple-macosx12.0".to_string(),
            data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".to_string(),
            os:   Os::MacOs,
            arch: Arch::Aarch64,
            env:  Env::None,
        };

        // Fallback to Windows triple if platform detection fails.
        #[allow(unreachable_code)]
        windows_x64::windows_x86_64_msvc()
    }

    /// Parse a target from a triple string (partial support — extend as needed).
    pub fn from_triple(triple: &str) -> Result<Self, String> {
        match triple {
            "x86_64-pc-windows-msvc" => Ok(windows_x64::windows_x86_64_msvc()),
            "x86_64-unknown-linux-gnu" => Ok(Self {
                triple:      triple.to_string(),
                data_layout: "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128".to_string(),
                os:   Os::Linux,
                arch: Arch::X86_64,
                env:  Env::Gnu,
            }),
            "aarch64-apple-macosx12.0" | "arm64-apple-macosx12.0" => Ok(Self {
                triple:      triple.to_string(),
                data_layout: "e-m:o-i64:64-i128:128-n32:64-S128".to_string(),
                os:   Os::MacOs,
                arch: Arch::Aarch64,
                env:  Env::None,
            }),
            _ => Err(format!("Unknown target triple '{}'. Supported: x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu, arm64-apple-macosx12.0", triple)),
        }
    }

    pub fn is_windows(&self) -> bool { self.os == Os::Windows }
    pub fn is_linux(&self)   -> bool { self.os == Os::Linux }
    pub fn is_macos(&self)   -> bool { self.os == Os::MacOs }

    pub fn default_exe_suffix(&self) -> &'static str {
        if self.is_windows() { ".exe" } else { "" }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.triple)
    }
}
