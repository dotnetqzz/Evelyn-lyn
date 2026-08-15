// target/diagnostics.rs — Toolchain-related user-facing diagnostics

/// Emit a helpful message when the LLVM toolchain is not found.
pub fn toolchain_not_found_message(search_paths: &[String]) -> String {
    let mut msg = String::new();
    msg.push_str("Avelyn compiler: LLVM/Clang toolchain not found.\n\n");
    msg.push_str("Searched the following locations:\n");
    for path in search_paths {
        msg.push_str(&format!("  • {}\n", path));
    }
    msg.push_str("\nTo fix this, do one of the following:\n");
    msg.push_str("  1. Install LLVM from https://releases.llvm.org/download.html\n");
    msg.push_str("     (check \"Add LLVM to the system PATH\" during installation)\n");
    msg.push_str("  2. Set the AVELYN_LLVM_PATH environment variable:\n");
    msg.push_str("       set AVELYN_LLVM_PATH=C:\\Program Files\\LLVM\\bin\n");
    msg.push_str("  3. Pass the path explicitly:\n");
    msg.push_str("       avelyn compile file.lyn --llvm-path \"C:\\Program Files\\LLVM\\bin\"\n");
    msg
}

/// Emit a helpful message when clang compilation fails.
pub fn clang_failed_message(clang_path: &str, exit_code: Option<i32>) -> String {
    let code_str = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
    format!(
        "Avelyn compiler: clang compilation failed (exit code: {}).\n\
         Clang path: {}\n\
         Tip: check that clang is correctly installed and the target triple matches.\n\
         Use --verbose to see the exact clang invocation.",
        code_str, clang_path
    )
}

/// Emit a helpful message when linking fails.
pub fn link_failed_message(linker: &str, exit_code: Option<i32>) -> String {
    let code_str = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
    format!(
        "Avelyn compiler: linker '{}' failed (exit code: {}).\n\
         Tip: ensure the Windows SDK is installed and MSVC CRT is accessible.",
        linker, code_str
    )
}
