// target/windows_x64.rs — Windows x86_64 MSVC target configuration
//
// Implements toolchain probing for Windows, searching for clang in:
//   1. The `--llvm-path` / `AVELYN_LLVM_PATH` environment variable.
//   2. Well-known LLVM install directories on Windows.
//   3. The system PATH.

use std::path::{Path, PathBuf};
use super::Target;

/// The Windows x86_64 MSVC target triple.
pub const TRIPLE: &str = "x86_64-pc-windows-msvc";

/// The LLVM data layout for Windows x86_64.
pub const DATA_LAYOUT: &str =
    "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128";

/// Construct the Windows x86_64 MSVC target descriptor.
pub fn windows_x86_64_msvc() -> Target {
    Target {
        triple:      TRIPLE.to_string(),
        data_layout: DATA_LAYOUT.to_string(),
        os:          super::Os::Windows,
        arch:        super::Arch::X86_64,
        env:         super::Env::Msvc,
    }
}

/// Probe the system for a usable clang executable.
///
/// Search order (recommendation from the implementation plan):
///   1. `AVELYN_LLVM_PATH` env var  (directory containing clang.exe)
///   2. `--llvm-path` flag override (passed in as `override_path`)
///   3. Well-known Windows install locations
///   4. System PATH
///
/// Returns `Ok(PathBuf)` with the clang executable path, or
/// `Err(Vec<String>)` with all locations that were searched.
pub fn probe_clang(override_path: Option<&str>) -> Result<PathBuf, Vec<String>> {
    let mut searched: Vec<String> = Vec::new();

    // Helper: check whether `dir/clang.exe` (or just `clang` on non-Windows) exists.
    let check_dir = |dir: &Path, searched: &mut Vec<String>| -> Option<PathBuf> {
        let exe = if cfg!(windows) {
            dir.join("clang.exe")
        } else {
            dir.join("clang")
        };
        searched.push(exe.display().to_string());
        if exe.exists() { Some(exe) } else { None }
    };

    // Priority 1: explicit override (from --llvm-path CLI flag).
    if let Some(p) = override_path {
        let dir = PathBuf::from(p);
        if let Some(exe) = check_dir(&dir, &mut searched) {
            return Ok(exe);
        }
    }

    // Priority 2: AVELYN_LLVM_PATH environment variable.
    if let Ok(env_path) = std::env::var("AVELYN_LLVM_PATH") {
        let dir = PathBuf::from(&env_path);
        if let Some(exe) = check_dir(&dir, &mut searched) {
            return Ok(exe);
        }
    }

    // Priority 3: Well-known Windows LLVM install locations.
    let well_known: &[&str] = &[
        r"C:\Program Files\LLVM\bin",
        r"C:\Program Files (x86)\LLVM\bin",
        r"C:\LLVM\bin",
        r"C:\tools\llvm\bin",
        // Chocolatey / scoop / winget typical paths
        r"C:\ProgramData\chocolatey\lib\llvm\tools\llvm\bin",
        r"C:\Users\Public\scoop\apps\llvm\current\bin",
    ];

    for wk in well_known {
        let dir = PathBuf::from(wk);
        if let Some(exe) = check_dir(&dir, &mut searched) {
            return Ok(exe);
        }
    }

    // Priority 4: search PATH via which/where.
    let clang_name = if cfg!(windows) { "clang.exe" } else { "clang" };
    if let Ok(path_var) = std::env::var("PATH") {
        for entry in std::env::split_paths(&path_var) {
            let candidate = entry.join(clang_name);
            searched.push(candidate.display().to_string());
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(searched)
}

/// Compile an LLVM IR file and C runtime source into an executable.
///
/// Uses clang as both the compiler and linker, which handles the Windows SDK
/// and CRT automatically when the MSVC target triple is specified.
pub fn compile_and_link(
    clang:       &Path,
    ir_file:     &Path,
    rt_c_file:   &Path,
    out_file:    &Path,
    opt_level:   u8,
    verbose:     bool,
) -> Result<(), String> {
    let opt_flag = format!("-O{}", opt_level.min(3));

    let mut cmd = std::process::Command::new(clang);
    cmd.arg(&opt_flag)
       .arg("-target").arg(TRIPLE)
       .arg(ir_file)
       .arg(rt_c_file)
       .arg("-o").arg(out_file);

    // On Windows, link against the CRT.
    if cfg!(windows) {
        cmd.arg("-lmsvcrt");
    }

    if verbose {
        eprintln!("[avelyn driver] Invoking: {:?}", cmd);
    }

    let status = cmd.status().map_err(|e| {
        format!("Failed to execute clang '{}': {}. {}",
            clang.display(), e,
            super::diagnostics::toolchain_not_found_message(&[clang.display().to_string()]))
    })?;

    if status.success() {
        Ok(())
    } else {
        Err(super::diagnostics::clang_failed_message(
            &clang.display().to_string(),
            status.code(),
        ))
    }
}

/// Compile an LLVM IR file to an object file.
pub fn compile_to_object(
    clang:     &Path,
    ir_file:   &Path,
    obj_file:  &Path,
    opt_level: u8,
    verbose:   bool,
) -> Result<(), String> {
    let opt_flag = format!("-O{}", opt_level.min(3));
    let mut cmd = std::process::Command::new(clang);
    cmd.arg(&opt_flag)
       .arg("-target").arg(TRIPLE)
       .arg("-c")
       .arg(ir_file)
       .arg("-o").arg(obj_file);

    if verbose {
        eprintln!("[avelyn driver] Invoking: {:?}", cmd);
    }

    let status = cmd.status().map_err(|e| {
        format!("Failed to execute clang: {}", e)
    })?;

    if status.success() { Ok(()) } else {
        Err(format!("clang -c failed with exit code {:?}", status.code()))
    }
}

/// Compile an LLVM IR file to assembly.
pub fn compile_to_asm(
    clang:     &Path,
    ir_file:   &Path,
    asm_file:  &Path,
    opt_level: u8,
    verbose:   bool,
) -> Result<(), String> {
    let opt_flag = format!("-O{}", opt_level.min(3));
    let mut cmd = std::process::Command::new(clang);
    cmd.arg(&opt_flag)
       .arg("-target").arg(TRIPLE)
       .arg("-S")
       .arg(ir_file)
       .arg("-o").arg(asm_file);

    if verbose {
        eprintln!("[avelyn driver] Invoking: {:?}", cmd);
    }

    let status = cmd.status().map_err(|e| format!("Failed to execute clang: {}", e))?;
    if status.success() { Ok(()) } else {
        Err(format!("clang -S failed with exit code {:?}", status.code()))
    }
}
