// stdlib/sys.rs — System/OS builtins

use std::rc::Rc;
use std::cell::RefCell;
use crate::value::{SylError, SylVal};
use crate::vm::Vm;

fn arg(args: &[SylVal], i: usize) -> SylVal {
    args.get(i).cloned().unwrap_or(SylVal::Null)
}

pub fn native_sys_platform(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    let p = if cfg!(target_os = "windows") { "windows" }
            else if cfg!(target_os = "macos") { "macos" }
            else { "linux" };
    Ok(SylVal::Str(Rc::new(p.to_string())))
}

pub fn native_sys_arch(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    let a = if cfg!(target_arch = "x86_64") { "x86_64" }
            else if cfg!(target_arch = "aarch64") { "aarch64" }
            else { "unknown" };
    Ok(SylVal::Str(Rc::new(a.to_string())))
}

pub fn native_sys_argv(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    let argv: Vec<SylVal> = std::env::args().map(|a| SylVal::Str(Rc::new(a))).collect();
    Ok(SylVal::List(Rc::new(RefCell::new(argv))))
}

pub fn native_sys_env(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let key = arg(args, 0).format();
    match std::env::var(&key) {
        Ok(v) => Ok(SylVal::Str(Rc::new(v))),
        Err(_) => Ok(SylVal::Null),
    }
}

pub fn native_sys_execute(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let cmd = arg(args, 0).format();
    let output = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", &cmd]).output()
    } else {
        std::process::Command::new("sh").args(["-c", &cmd]).output()
    };
    match output {
        Ok(out) => Ok(SylVal::Str(Rc::new(String::from_utf8_lossy(&out.stdout).to_string()))),
        Err(e) => Ok(SylVal::Str(Rc::new(e.to_string()))),
    }
}

pub fn native_sys_remove_file(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let path = arg(args, 0).format();
    let ok = std::fs::remove_file(&path).is_ok();
    Ok(SylVal::Bool(ok))
}

pub fn native_sys_copy_file(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let src = arg(args, 0).format();
    let dst = arg(args, 1).format();
    let ok = std::fs::copy(&src, &dst).is_ok();
    Ok(SylVal::Bool(ok))
}

pub fn native_sys_move_file(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let src = arg(args, 0).format();
    let dst = arg(args, 1).format();
    let ok = std::fs::rename(&src, &dst).is_ok();
    Ok(SylVal::Bool(ok))
}

pub fn native_sys_secure_random_double(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    // Simple LCG fallback (no external dep)
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let v = (seed as f64) / (u32::MAX as f64);
    Ok(SylVal::Float(v))
}

pub fn native_sys_secure_random_bytes(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let n = arg(args, 0).as_i64() as usize;
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as u64;
    let mut state = seed;
    let bytes: Vec<SylVal> = (0..n).map(|_| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        SylVal::Int((state >> 33) as i64 & 0xFF)
    }).collect();
    Ok(SylVal::List(Rc::new(RefCell::new(bytes))))
}

pub fn native_sys_readline(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    let line = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
    Ok(SylVal::Str(Rc::new(line)))
}

// Simple regex: only literal substring match (no regex engine dep)
pub fn native_sys_regex_match(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = arg(args, 0).format();
    let pat = arg(args, 1).format();
    Ok(SylVal::Bool(s.contains(pat.as_str())))
}

pub fn native_sys_regex_replace(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = arg(args, 0).format();
    let pat = arg(args, 1).format();
    let repl = arg(args, 2).format();
    Ok(SylVal::Str(Rc::new(s.replace(pat.as_str(), &repl))))
}

pub fn native_sys_regex_groups(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::List(Rc::new(RefCell::new(vec![]))))
}

pub fn native_sys_regex_find_all(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::List(Rc::new(RefCell::new(vec![]))))
}

pub fn native_url_encode(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = arg(args, 0).format();
    let encoded: String = s.chars().map(|c| {
        if c.is_alphanumeric() || "-_.~".contains(c) { c.to_string() }
        else { format!("%{:02X}", c as u32) }
    }).collect();
    Ok(SylVal::Str(Rc::new(encoded)))
}

pub fn native_url_decode(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = arg(args, 0).format();
    // Simple percent-decode
    let mut result = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or(""), 16) {
                result.push(hex as char);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' { result.push(' '); } else { result.push(bytes[i] as char); }
        i += 1;
    }
    Ok(SylVal::Str(Rc::new(result)))
}

pub fn native_sys_url_parse(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    use indexmap::IndexMap;
    let url = arg(args, 0).format();
    let mut map = IndexMap::new();
    map.insert("raw".to_string(), SylVal::Str(Rc::new(url)));
    Ok(SylVal::Map(Rc::new(RefCell::new(map))))
}

pub fn register(vm: &mut Vm) {
    vm.register_native("sysPlatform",           native_sys_platform);
    vm.register_native("sysArch",               native_sys_arch);
    vm.register_native("sysArgv",               native_sys_argv);
    vm.register_native("sysEnv",                native_sys_env);
    vm.register_native("sysExecute",            native_sys_execute);
    vm.register_native("sysRemoveFile",         native_sys_remove_file);
    vm.register_native("sysCopyFile",           native_sys_copy_file);
    vm.register_native("sysMoveFile",           native_sys_move_file);
    vm.register_native("sysSecureRandomDouble", native_sys_secure_random_double);
    vm.register_native("sysSecureRandomBytes",  native_sys_secure_random_bytes);
    vm.register_native("sysReadLine",           native_sys_readline);
    vm.register_native("sysRegexMatch",         native_sys_regex_match);
    vm.register_native("sysRegexReplace",       native_sys_regex_replace);
    vm.register_native("sysRegexGroups",        native_sys_regex_groups);
    vm.register_native("sysRegexFindAll",       native_sys_regex_find_all);
    vm.register_native("sysUrlParse",           native_sys_url_parse);
    vm.register_native("urlEncode",             native_url_encode);
    vm.register_native("urlDecode",             native_url_decode);
}
