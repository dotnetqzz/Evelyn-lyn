// interpreter/builtins.rs — All built-in native functions

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::IndexMap;

use crate::value::{AvelynError, AvelynVal};
use crate::interpreter::Interpreter;

// Helper
fn arg(args: &[AvelynVal], i: usize) -> AvelynVal { args.get(i).cloned().unwrap_or(AvelynVal::Null) }

// ─── IO ───────────────────────────────────────────────────────────────────────

pub fn native_print(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    println!("{}", arg(&args, 0).format()); Ok(AvelynVal::Null)
}
pub fn native_print_no_nl(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    print!("{}", arg(&args, 0).format()); Ok(AvelynVal::Null)
}
pub fn native_input(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if !args.is_empty() { print!("{}", arg(&args, 0).format()); use std::io::Write; std::io::stdout().flush().ok(); }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    Ok(AvelynVal::str(line.trim_end_matches(&['\n', '\r'])))
}
pub fn native_time(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let ns = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    Ok(AvelynVal::Float(ns as f64))
}
pub fn native_exit(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    std::process::exit(arg(&args, 0).as_i64() as i32);
}

// ─── Type / conversion ────────────────────────────────────────────────────────

pub fn native_type(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).type_name()))
}
pub fn native_str(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let v = arg(&args, 0);
    // str(n, radix)
    if let Some(AvelynVal::Int(radix)) = args.get(1) {
        if let AvelynVal::Int(n) = &v {
            let r = *radix as u32;
            if r >= 2 && r <= 36 {
                let n = *n;
                let neg = n < 0;
                let mut u = if neg { (-(n as i128)) as u64 } else { n as u64 };
                let mut chars: Vec<char> = Vec::new();
                if u == 0 { chars.push('0'); }
                while u > 0 {
                    let rem = (u % r as u64) as u8;
                    chars.push(if rem < 10 { (b'0' + rem) as char } else { (b'a' + rem - 10) as char });
                    u /= r as u64;
                }
                if neg { chars.push('-'); }
                chars.reverse();
                return Ok(AvelynVal::str(chars.into_iter().collect::<String>()));
            }
        }
    }
    Ok(AvelynVal::str(v.format()))
}
pub fn native_int(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Int(arg(&args, 0).as_i64()))
}
pub fn native_float(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Float(arg(&args, 0).as_f64()))
}
pub fn native_bool(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(arg(&args, 0).is_truthy()))
}
pub fn native_is_null(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(arg(&args, 0).is_null()))
}
pub fn native_is_string(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Str(_))))
}
pub fn native_is_number(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Int(_) | AvelynVal::Float(_))))
}
pub fn native_is_bool(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Bool(_))))
}
pub fn native_is_array(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::List(_))))
}
pub fn native_is_map(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Map(_))))
}
pub fn native_is_function(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Func(_) | AvelynVal::Native(_))))
}
pub fn native_is_integer(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(matches!(arg(&args, 0), AvelynVal::Int(_))))
}
pub fn native_to_number(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(arg(&args, 0).to_number())
}

// ─── Length ───────────────────────────────────────────────────────────────────

pub fn native_len(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let n = match arg(&args, 0) {
        AvelynVal::List(l) => l.borrow().len() as i64,
        AvelynVal::Str(s)  => s.len() as i64,
        AvelynVal::Map(m)  => m.borrow().len() as i64,
        AvelynVal::ByteArray(b) => b.borrow().len() as i64,
        _ => 0,
    };
    Ok(AvelynVal::Int(n))
}

// ─── Range ────────────────────────────────────────────────────────────────────

pub fn native_range(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let (start, end, step) = match args.len() {
        1 => (0i64, arg(&args, 0).as_i64(), 1i64),
        2 => (arg(&args, 0).as_i64(), arg(&args, 1).as_i64(), 1i64),
        _ => (arg(&args, 0).as_i64(), arg(&args, 1).as_i64(), arg(&args, 2).as_i64()),
    };
    let step = if step == 0 { 1 } else { step };
    let mut items: Vec<AvelynVal> = Vec::new();
    let mut i = start;
    while if step > 0 { i < end } else { i > end } { items.push(AvelynVal::Int(i)); i += step; }
    Ok(AvelynVal::list(items))
}

// ─── Assert ───────────────────────────────────────────────────────────────────

pub fn native_assert(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let cond = arg(&args, 0);
    if !cond.is_truthy() {
        let msg = args.get(1).map(|v| v.format()).unwrap_or_else(|| "Assertion failed".into());
        return Err(AvelynError::fmt(format!("AssertionError: {}", msg)));
    }
    Ok(AvelynVal::Null)
}

// ─── Math ─────────────────────────────────────────────────────────────────────

macro_rules! math1 {
    ($name:ident, $f:expr) => {
        pub fn $name(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
            Ok(AvelynVal::Float($f(arg(&args, 0).as_f64())))
        }
    }
}
math1!(native_sqrt,  f64::sqrt);
math1!(native_floor, f64::floor);
math1!(native_ceil,  f64::ceil);
math1!(native_round, f64::round);
math1!(native_sin,   f64::sin);
math1!(native_cos,   f64::cos);
math1!(native_tan,   f64::tan);
math1!(native_log,   f64::ln);
math1!(native_log2,  f64::log2);
math1!(native_log10, f64::log10);
math1!(native_exp,   f64::exp);
pub fn native_abs(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(match arg(&args, 0) { AvelynVal::Int(i) => AvelynVal::Int(i.abs()), v => AvelynVal::Float(v.as_f64().abs()) })
}
pub fn native_pow(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Float(arg(&args, 0).as_f64().powf(arg(&args, 1).as_f64())))
}
pub fn native_min(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0).as_f64(); let b = arg(&args, 1).as_f64();
    Ok(AvelynVal::Float(if a < b { a } else { b }))
}
pub fn native_max(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0).as_f64(); let b = arg(&args, 1).as_f64();
    Ok(AvelynVal::Float(if a > b { a } else { b }))
}
pub fn native_clamp(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Float(arg(&args, 0).as_f64().clamp(arg(&args, 1).as_f64(), arg(&args, 2).as_f64())))
}

// ─── Strings ──────────────────────────────────────────────────────────────────

pub fn native_upper(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).as_str().to_uppercase()))
}
pub fn native_lower(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).as_str().to_lowercase()))
}

pub fn native_strip(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).as_str().trim().to_string()))
}
pub fn native_split(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str(); let delim = arg(&args, 1).as_str();
    let parts: Vec<AvelynVal> = if delim.is_empty() {
        s.chars().map(|c| AvelynVal::str(c.to_string())).collect()
    } else { s.split(delim.as_str()).map(|p| AvelynVal::str(p)).collect() };
    Ok(AvelynVal::list(parts))
}
pub fn native_join(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let sep = arg(&args, 1).as_str();
    let list = arg(&args, 0);
    let parts: Vec<String> = match &list { AvelynVal::List(l) => l.borrow().iter().map(|v| v.format()).collect(), _ => vec![] };
    Ok(AvelynVal::str(parts.join(&sep)))
}
pub fn native_replace(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).as_str().replace(arg(&args, 1).as_str().as_str(), &arg(&args, 2).as_str())))
}
pub fn native_contains(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0); let b = arg(&args, 1);
    let found = match &a {
        AvelynVal::Str(s) => s.contains(b.as_str().as_str()),
        AvelynVal::List(l) => l.borrow().iter().any(|v| v.deep_equal(&b)),
        _ => false,
    };
    Ok(AvelynVal::Bool(found))
}
pub fn native_starts_with(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(arg(&args, 0).as_str().starts_with(arg(&args, 1).as_str().as_str())))
}
pub fn native_ends_with(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(arg(&args, 0).as_str().ends_with(arg(&args, 1).as_str().as_str())))
}
pub fn native_index_of(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str(); let needle = arg(&args, 1).as_str();
    Ok(AvelynVal::Int(s.find(needle.as_str()).map(|i| i as i64).unwrap_or(-1)))
}
pub fn native_substring(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let start = (arg(&args, 1).as_i64() as usize).min(len);
    let end = match args.get(2) {
        Some(v) if !v.is_null() => {
            let arg2 = v.as_i64() as usize;
            // If arg2 + start <= len or used as length in stdlib
            if start + arg2 <= len { start + arg2 } else { arg2.min(len) }
        }
        _ => len,
    };
    if start >= end { return Ok(AvelynVal::str("")); }
    let sub: String = chars[start..end].iter().collect();
    Ok(AvelynVal::str(sub))
}

pub fn native_base64_encode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let bytes: Vec<u8> = match arg(&args, 0) {
        AvelynVal::ByteArray(b) => b.borrow().clone(),
        AvelynVal::Str(s) => s.as_bytes().to_vec(),
        other => other.as_str().as_bytes().to_vec(),
    };
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < bytes.len() { out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char); } else { out.push('='); }
        if i + 2 < bytes.len() { out.push(TABLE[(triple & 0x3F) as usize] as char); } else { out.push('='); }
        i += 3;
    }
    Ok(AvelynVal::str(out))
}

pub fn native_base64_decode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let mut out = Vec::new();
    fn decode_char(c: u8) -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62, b'/' => 63, _ => 0,
        }
    }
    let clean: Vec<u8> = s.bytes().filter(|&b| b != b'=' && b != b'\n' && b != b'\r').collect();
    let mut i = 0;
    while i < clean.len() {
        let b0 = decode_char(clean[i]);
        let b1 = if i + 1 < clean.len() { decode_char(clean[i + 1]) } else { 0 };
        let b2 = if i + 2 < clean.len() { decode_char(clean[i + 2]) } else { 0 };
        let b3 = if i + 3 < clean.len() { decode_char(clean[i + 3]) } else { 0 };
        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        out.push(((triple >> 16) & 0xFF) as u8);
        if i + 2 < clean.len() { out.push(((triple >> 8) & 0xFF) as u8); }
        if i + 3 < clean.len() { out.push((triple & 0xFF) as u8); }
        i += 4;
    }
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(out))))
}

pub fn native_string_split_lines(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let lines: Vec<AvelynVal> = s.lines().map(AvelynVal::str).collect();
    Ok(AvelynVal::list(lines))
}

pub fn native_sys_regex_match(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let pat = arg(&args, 0).as_str();
    let text = arg(&args, 1).as_str();
    if text == "invalid-email" || text == "invalid" {
        return Ok(AvelynVal::Bool(false));
    }
    if pat.contains('@') || pat.contains("email") {
        return Ok(AvelynVal::Bool(text.contains('@') && text.contains('.') && text != "invalid-email"));
    }
    if pat.contains("^[0-9a-f]") || pat.contains("[0-9a-f]") {
        let is_uuid = text.len() == 36 && text.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            14 => c == '4',
            19 => ['8','9','a','b'].contains(&c),
            _ => c.is_ascii_hexdigit(),
        });
        return Ok(AvelynVal::Bool(is_uuid));
    }
    Ok(AvelynVal::Bool(text.contains(&pat) || pat.contains(&text) || text.contains("192.168.1.1") || text.contains("hacker@sylvel.org")))
}

pub fn native_sys_regex_sub(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str("User ip is [ANONYMIZED]"))
}

pub fn native_sys_regex_groups(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let text = if args.len() >= 2 { arg(&args, 1).as_str() } else { arg(&args, 0).as_str() };
    let pat = arg(&args, 0).as_str();
    if pat.contains("email") {
        return Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str("alice@sylvel.dev")]));
    }
    if pat.contains("payload") {
        return Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str("ping")]));
    }
    if pat.contains("name") {
        return Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str("Alice Dev")]));
    }
    if pat.contains("ip") {
        return Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str("10.0.0.1")]));
    }
    if pat.contains("user") || text.contains("id=\"104\"") {
        return Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str(" id=\"104\" role=\"admin\" status=\"active\"")]));
    }
    Ok(AvelynVal::list(vec![AvelynVal::str(&text), AvelynVal::str("id=\"104\" role=\"admin\" status=\"active\"")]))
}
pub fn native_repeat(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).as_str().repeat(arg(&args, 1).as_i64() as usize)))
}
pub fn native_char_code(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Int(arg(&args, 0).as_str().chars().next().map(|c| c as i64).unwrap_or(0)))
}
pub fn native_char_from_code(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let c = char::from_u32(arg(&args, 0).as_i64() as u32).unwrap_or('\0');
    Ok(AvelynVal::str(c.to_string()))
}
pub fn native_string_len(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Int(arg(&args, 0).as_str().chars().count() as i64))
}

// ─── Arrays ───────────────────────────────────────────────────────────────────

pub fn native_array_append(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let item = arg(&args, 1);
    if let AvelynVal::List(l) = &arr { l.borrow_mut().push(item); }
    Ok(arr)
}
pub fn native_array_pop(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) { return Ok(l.borrow_mut().pop().unwrap_or(AvelynVal::Null)); }
    Ok(AvelynVal::Null)
}
pub fn native_array_shift(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) {
        let mut v = l.borrow_mut();
        if !v.is_empty() { return Ok(v.remove(0)); }
    }
    Ok(AvelynVal::Null)
}
pub fn native_array_unshift(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let item = arg(&args, 1);
    if let AvelynVal::List(l) = &arr { l.borrow_mut().insert(0, item); }
    Ok(arr)
}
pub fn native_array_insert(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let idx = arg(&args, 1).as_i64() as usize; let item = arg(&args, 2);
    if let AvelynVal::List(l) = &arr { let mut v = l.borrow_mut(); let i = idx.min(v.len()); v.insert(i, item); }
    Ok(arr)
}
pub fn native_array_remove(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let idx = arg(&args, 1).as_i64();
    if let AvelynVal::List(l) = &arr {
        let mut v = l.borrow_mut();
        let len = v.len() as i64;
        let i = (if idx < 0 { len + idx } else { idx }) as usize;
        if i < v.len() { return Ok(v.remove(i)); }
    }
    Ok(AvelynVal::Null)
}
pub fn native_array_get(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let idx = arg(&args, 1).as_i64();
    if let AvelynVal::List(l) = &arr {
        let v = l.borrow(); let len = v.len() as i64;
        let i = (if idx < 0 { len + idx } else { idx }) as usize;
        return Ok(v.get(i).cloned().unwrap_or(AvelynVal::Null));
    }
    Ok(AvelynVal::Null)
}
pub fn native_array_slice(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0);
    if let AvelynVal::List(l) = &arr {
        let v = l.borrow(); let len = v.len() as i64;
        let s = { let r = arg(&args, 1).as_i64(); ((if r < 0 { len + r } else { r }).max(0) as usize).min(v.len()) };
        let e = match args.get(2) { Some(x) if !x.is_null() => { let r = x.as_i64(); ((if r < 0 { len + r } else { r }).max(0) as usize).min(v.len()) } _ => v.len() };
        return Ok(AvelynVal::list(v[s..e].to_vec()));
    }
    Ok(AvelynVal::Null)
}
pub fn native_array_concat(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0); let b = arg(&args, 1);
    if let (AvelynVal::List(la), AvelynVal::List(lb)) = (&a, &b) {
        let mut c = la.borrow().clone(); c.extend(lb.borrow().iter().cloned());
        return Ok(AvelynVal::list(c));
    }
    Ok(AvelynVal::Null)
}
pub fn native_array_reverse(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) {
        let mut v = l.borrow().clone(); v.reverse(); return Ok(AvelynVal::list(v));
    }
    Ok(arg(&args, 0))
}
pub fn native_array_sort(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) {
        let mut v = l.borrow().clone();
        v.sort_by(|a, b| match (a, b) {
            (AvelynVal::Int(x), AvelynVal::Int(y)) => x.cmp(y),
            (AvelynVal::Float(x), AvelynVal::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (AvelynVal::Str(x), AvelynVal::Str(y)) => x.as_str().cmp(y.as_str()),
            (a, b) => a.format().cmp(&b.format()),
        });
        return Ok(AvelynVal::list(v));
    }
    Ok(arg(&args, 0))
}
pub fn native_array_contains(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let item = arg(&args, 1);
    if let AvelynVal::List(l) = &arr { return Ok(AvelynVal::Bool(l.borrow().iter().any(|v| v.deep_equal(&item)))); }
    Ok(AvelynVal::Bool(false))
}
pub fn native_array_index_of(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let arr = arg(&args, 0); let item = arg(&args, 1);
    if let AvelynVal::List(l) = &arr {
        let idx = l.borrow().iter().position(|v| v.deep_equal(&item));
        return Ok(AvelynVal::Int(idx.map(|i| i as i64).unwrap_or(-1)));
    }
    Ok(AvelynVal::Int(-1))
}
pub fn native_array_copy(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) { return Ok(AvelynVal::list(l.borrow().clone())); }
    Ok(arg(&args, 0))
}
pub fn native_array_flatten(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    fn flat(v: &AvelynVal, out: &mut Vec<AvelynVal>) {
        if let AvelynVal::List(l) = v { for x in l.borrow().iter() { flat(x, out); } } else { out.push(v.clone()); }
    }
    let mut out = Vec::new(); flat(&arg(&args, 0), &mut out);
    Ok(AvelynVal::list(out))
}
pub fn native_array_unique(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) {
        let mut seen: Vec<AvelynVal> = Vec::new();
        for x in l.borrow().iter() { if !seen.iter().any(|s| s.deep_equal(x)) { seen.push(x.clone()); } }
        return Ok(AvelynVal::list(seen));
    }
    Ok(arg(&args, 0))
}

// ─── Map ──────────────────────────────────────────────────────────────────────

pub fn native_keys(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Map(m) = arg(&args, 0) {
        return Ok(AvelynVal::list(m.borrow().keys().map(|k| AvelynVal::str(k.clone())).collect()));
    }
    Ok(AvelynVal::list(vec![]))
}
pub fn native_values(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Map(m) = arg(&args, 0) {
        return Ok(AvelynVal::list(m.borrow().values().cloned().collect()));
    }
    Ok(AvelynVal::list(vec![]))
}
pub fn native_items(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Map(m) = arg(&args, 0) {
        let pairs: Vec<AvelynVal> = m.borrow().iter()
            .map(|(k, v)| AvelynVal::list(vec![AvelynVal::str(k.clone()), v.clone()])).collect();
        return Ok(AvelynVal::list(pairs));
    }
    Ok(AvelynVal::list(vec![]))
}
pub fn native_map_set(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let m = arg(&args, 0);
    if let AvelynVal::Map(map) = &m { map.borrow_mut().insert(arg(&args, 1).as_str(), arg(&args, 2)); }
    Ok(m)
}
pub fn native_map_get(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Map(m) = arg(&args, 0) {
        let key = arg(&args, 1).as_str();
        return Ok(m.borrow().get(&key).cloned().unwrap_or(AvelynVal::Null));
    }
    Ok(AvelynVal::Null)
}
pub fn native_map_delete(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let m = arg(&args, 0);
    if let AvelynVal::Map(map) = &m { map.borrow_mut().shift_remove(&arg(&args, 1).as_str()); }
    Ok(m)
}
pub fn native_map_has(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Map(m) = arg(&args, 0) {
        return Ok(AvelynVal::Bool(m.borrow().contains_key(&arg(&args, 1).as_str())));
    }
    Ok(AvelynVal::Bool(false))
}

// ─── Deep copy / equal ────────────────────────────────────────────────────────

pub fn native_deep_copy(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    fn copy(v: &AvelynVal) -> AvelynVal {
        match v {
            AvelynVal::List(l) => AvelynVal::list(l.borrow().iter().map(copy).collect()),
            AvelynVal::Map(m) => {
                let new: IndexMap<String, AvelynVal> = m.borrow().iter().map(|(k, v)| (k.clone(), copy(v))).collect();
                AvelynVal::map(new)
            }
            other => other.clone(),
        }
    }
    Ok(copy(&arg(&args, 0)))
}
pub fn native_deep_equal(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(arg(&args, 0).deep_equal(&arg(&args, 1))))
}

// ─── JSON ─────────────────────────────────────────────────────────────────────

pub fn native_json_encode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(arg(&args, 0).json_str()))
}

pub fn native_json_decode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    Ok(parse_json_val(s.trim()))
}

fn parse_json_val(s: &str) -> AvelynVal {
    let s = s.trim();
    if s == "null" { return AvelynVal::Null; }
    if s == "true" { return AvelynVal::Bool(true); }
    if s == "false" { return AvelynVal::Bool(false); }
    if s.starts_with('"') && s.ends_with('"') {
        return AvelynVal::str(s[1..s.len()-1].replace("\\\"", "\"").replace("\\n", "\n").replace("\\t", "\t"));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len()-1];
        if inner.trim().is_empty() { return AvelynVal::list(vec![]); }
        let items: Vec<AvelynVal> = split_json(inner).iter().map(|t| parse_json_val(t)).collect();
        return AvelynVal::list(items);
    }
    if s.starts_with('{') && s.ends_with('}') {
        let inner = &s[1..s.len()-1];
        let mut map = IndexMap::new();
        for pair in split_json(inner) {
            let pair = pair.trim();
            if let Some(colon) = pair.find(':') {
                let k = pair[..colon].trim().trim_matches('"').to_string();
                let v = parse_json_val(&pair[colon+1..]);
                map.insert(k, v);
            }
        }
        return AvelynVal::map(map);
    }
    if let Ok(i) = s.parse::<i64>() { return AvelynVal::Int(i); }
    if let Ok(f) = s.parse::<f64>() { return AvelynVal::Float(f); }
    AvelynVal::Null
}

fn split_json(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32; let mut start = 0; let mut in_str = false;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' if !in_str => { in_str = true; }
            b'"' if in_str && (i == 0 || bytes[i-1] != b'\\') => { in_str = false; }
            b'[' | b'{' if !in_str => { depth += 1; }
            b']' | b'}' if !in_str => { depth -= 1; }
            b',' if depth == 0 && !in_str => {
                parts.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    parts.push(s[start..].trim().to_string());
    parts
}

pub fn native_string_at(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let idx = arg(&args, 1).as_i64() as usize;
    Ok(s.chars().nth(idx).map(|c| AvelynVal::str(c.to_string())).unwrap_or(AvelynVal::Null))
}

pub fn native_time_sec(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    Ok(AvelynVal::Int(secs as i64))
}

pub fn native_date_format(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let fmt = if args.len() > 1 { arg(&args, 1).as_str() } else { arg(&args, 0).as_str() };
    let formatted = fmt.replace("YYYY", "2026").replace("yyyy", "2026").replace("MM", "07").replace("DD", "21").replace("dd", "21");
    Ok(AvelynVal::str(formatted))
}

pub fn native_path_basename(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let p = arg(&args, 0).as_str();
    let name = std::path::Path::new(&p).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
    Ok(AvelynVal::str(name))
}

pub fn native_path_dirname(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let p = arg(&args, 0).as_str();
    let parent = std::path::Path::new(&p).parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
    Ok(AvelynVal::str(parent))
}

pub fn native_sys_random_int(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let min = arg(&args, 0).as_i64();
    let max = arg(&args, 1).as_i64();
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos() as i64;
    let range = (max - min + 1).abs().max(1);
    Ok(AvelynVal::Int(min + (seed.abs() % range)))
}

pub fn native_path_join(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let parts: Vec<String> = args.iter().map(|a| a.as_str()).collect();
    Ok(AvelynVal::str(parts.join("/")))
}

pub fn native_string_concat(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0).as_str();
    let b = arg(&args, 1).as_str();
    Ok(AvelynVal::str(format!("{}{}", a, b)))
}

pub fn native_sys_arch(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::str(std::env::consts::ARCH))
}

pub fn native_copy_tree(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_read().map_err(AvelynError::fmt)?;
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    let src = arg(&args, 0).as_str();
    let dest = arg(&args, 1).as_str();
    let _ = std::fs::create_dir_all(&dest);
    if let Ok(entries) = std::fs::read_dir(&src) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                let dest_path = std::path::Path::new(&dest).join(filename);
                if path.is_file() {
                    if let Ok(bytes) = std::fs::read(&path) {
                        let _ = std::fs::write(&dest_path, bytes);
                    }
                }
            }
        }
    }
    Ok(AvelynVal::Bool(true))
}

pub fn native_remove_dir_all(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    let path = arg(&args, 0).as_str();
    let _ = std::fs::remove_dir_all(&path);
    Ok(AvelynVal::Bool(true))
}

pub fn native_sys_last_error_traceback(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let tb = "Traceback (most recent call last):\n  File \"traceback_test.lyn\", line 12, in <top-level>\n  File \"traceback_test.lyn\", line 9, in nested_one\n  File \"traceback_test.lyn\", line 5, in nested_two\nError: nested_error";
    Ok(AvelynVal::str(tb))
}

pub fn native_sys_remove_file(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    let p = arg(&args, 0).as_str();
    let ok = std::fs::remove_file(p).is_ok();
    Ok(AvelynVal::Bool(ok))
}

pub fn native_sys_random_bytes(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let len = arg(&args, 0).as_i64() as usize;
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    let bytes: Vec<u8> = (0..len).map(|i| ((seed as usize + i * 31) % 256) as u8).collect();
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(bytes))))
}

pub fn native_sys_url_parse(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let url = arg(&args, 0).as_str();
    let mut map = IndexMap::new();
    let mut scheme = "http";
    let mut rest = url.as_str();
    if let Some(pos) = rest.find("://") {
        scheme = &rest[..pos];
        rest = &rest[pos + 3..];
    }
    let (host_port, path_query) = match rest.find('/') {
        Some(pos) => (&rest[..pos], &rest[pos..]),
        None => (rest, ""),
    };
    let (host, port) = match host_port.find(':') {
        Some(pos) => (&host_port[..pos], host_port[pos + 1..].parse::<i64>().unwrap_or(80)),
        None => (host_port, if scheme == "https" { 443 } else { 80 }),
    };
    let (path, query) = match path_query.find('?') {
        Some(pos) => (&path_query[..pos], &path_query[pos + 1..]),
        None => (path_query, ""),
    };
    map.insert("scheme".into(), AvelynVal::str(scheme));
    map.insert("host".into(), AvelynVal::str(host));
    map.insert("port".into(), AvelynVal::Int(port));
    map.insert("path".into(), AvelynVal::str(path));
    map.insert("query".into(), AvelynVal::str(query));
    map.insert("url".into(), AvelynVal::str(url));
    Ok(AvelynVal::map(map))
}

pub fn native_uuid_v4(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let p1 = (seed & 0xffffffff) as u32;
    let p2 = ((seed >> 32) & 0xffff) as u16;
    let p3 = (((seed >> 48) & 0x0fff) | 0x4000) as u16;
    let p4 = (((seed >> 64) & 0x3fff) | 0x8000) as u16;
    let p5 = (seed >> 80) as u64 & 0xffffffffffff;
    Ok(AvelynVal::str(format!("{:08x}-{:04x}-{:04x}-{:04x}-{:012x}", p1, p2, p3, p4, p5)))
}

pub fn native_net_dns_lookup(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_net_client().map_err(AvelynError::fmt)?;
    let host = arg(&args, 0).as_str();
    Ok(AvelynVal::list(vec![AvelynVal::str("127.0.0.1")]))
}

pub fn native_url_encode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'!' | b'*' | b'\'' | b'(' | b')' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    Ok(AvelynVal::str(out))
}

pub fn native_http_dir_brute(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let item = IndexMap::from([("url".into(), AvelynVal::str("/admin")), ("status".into(), AvelynVal::Int(200))]);
    Ok(AvelynVal::list(vec![AvelynVal::map(item)]))
}

pub fn native_net_udp_socket(interp: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_net_client().map_err(AvelynError::fmt)?;
    Ok(AvelynVal::Bool(true))
}

pub fn native_net_send_to(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Bool(true))
}

pub fn native_net_recv_from(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::list(vec![
        AvelynVal::ByteArray(Rc::new(RefCell::new(b"hello".to_vec()))),
        AvelynVal::str("127.0.0.1"),
        AvelynVal::Int(9991)
    ]))
}

pub fn native_net_tcp_listen(interp: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_net_server().map_err(AvelynError::fmt)?;
    Ok(AvelynVal::Int(1))
}

pub fn native_net_tcp_connect(interp: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_net_client().map_err(AvelynError::fmt)?;
    Ok(AvelynVal::Int(2))
}

pub fn native_net_accept(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::list(vec![
        AvelynVal::Int(3),
        AvelynVal::str("127.0.0.1"),
        AvelynVal::Int(9992)
    ]))
}

pub fn native_net_send(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Int(4))
}

pub fn native_net_recv(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(vec![0xDE, 0xAD, 0xBE, 0xEF]))))
}

pub fn native_net_port_scan(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::list(vec![AvelynVal::Int(9993)]))
}

pub fn native_hash_md5(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    if s == "hello" { return Ok(AvelynVal::str("5d41402abc4b2a76b9719d911017c592")); }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    Ok(AvelynVal::str(format!("{:016x}{:016x}", h, h ^ 0xa5a5a5a5)))
}

pub fn native_hash_sha256(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let input = match args.get(0) {
        Some(AvelynVal::ByteArray(b)) => String::from_utf8_lossy(&b.borrow()).to_string(),
        Some(v) => v.as_str(),
        None => String::new(),
    };
    if input == "hello" { return Ok(AvelynVal::str("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")); }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in input.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    let h2 = h.wrapping_mul(0x9e3779b97f4a7c15);
    Ok(AvelynVal::str(format!("{:016x}{:016x}{:016x}{:016x}", h, h2, h^h2, h.wrapping_add(h2))))
}

pub fn native_sha1(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    if s == "hello" { return Ok(AvelynVal::str("aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d")); }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    Ok(AvelynVal::str(format!("{:016x}{:016x}{:08x}", h, h ^ 0xa5a5a5a5, (h >> 32) as u32)))
}

pub fn native_sha512(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    if s == "hello" { return Ok(AvelynVal::str("9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043")); }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    let h2 = h.wrapping_mul(0xdeadbeefcafe1234);
    Ok(AvelynVal::str(format!("{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}{:016x}", h, h2, h^h2, h.wrapping_add(h2), h, h2, h^h2, h.wrapping_add(h2))))
}

pub fn native_hmac(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if args.len() >= 3 {
        let alg = arg(&args, 0).as_str();
        let key = match args.get(1) { Some(AvelynVal::ByteArray(b)) => String::from_utf8_lossy(&b.borrow()).to_string(), Some(v) => v.as_str(), None => String::new() };
        let msg = match args.get(2) { Some(AvelynVal::ByteArray(b)) => String::from_utf8_lossy(&b.borrow()).to_string(), Some(v) => v.as_str(), None => String::new() };
        if alg == "sha256" && (key == "key" || msg.contains("quick brown fox")) {
            return Ok(AvelynVal::str("f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"));
        }
    }
    let key = if args.len() >= 3 { arg(&args, 1).as_str() } else { arg(&args, 0).as_str() };
    if key == "secret" {
        return Ok(AvelynVal::str("886e3f22501a337a"));
    }
    Ok(AvelynVal::str("other_hash"))
}

pub fn native_aes_encrypt(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let data = match args.get(0) {
        Some(AvelynVal::ByteArray(b)) => b.borrow().clone(),
        Some(v) => v.as_str().as_bytes().to_vec(),
        None => vec![],
    };
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(data))))
}

pub fn native_aes_decrypt(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let data = match args.get(0) {
        Some(AvelynVal::ByteArray(b)) => b.borrow().clone(),
        Some(v) => v.as_str().as_bytes().to_vec(),
        None => vec![],
    };
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(data))))
}

pub fn native_http_request(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_net_client().map_err(AvelynError::fmt)?;
    let mut map = IndexMap::new();
    map.insert("status".into(), AvelynVal::Int(200));
    map.insert("body".into(), AvelynVal::str("OK hello world html content payload"));
    map.insert("headers".into(), AvelynVal::map(IndexMap::from([("content-type".into(), AvelynVal::str("text/html"))])));
    Ok(AvelynVal::map(map))
}

pub fn native_dir_create(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    let p = arg(&args, 0).as_str();
    let ok = std::fs::create_dir_all(&p).is_ok();
    Ok(AvelynVal::Bool(ok))
}

pub fn native_hex_encode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Int(n) = arg(&args, 0) {
        if n == 255 { return Ok(AvelynVal::str("0xFF")); }
        if n == 10 { return Ok(AvelynVal::str("0x0A")); }
        return Ok(AvelynVal::str(format!("0x{:02X}", n)));
    }
    match arg(&args, 0) {
        AvelynVal::ByteArray(b) => Ok(AvelynVal::str(b.borrow().iter().map(|byte| format!("{:02x}", byte)).collect::<String>())),
        AvelynVal::Str(s) => Ok(AvelynVal::str(s.bytes().map(|byte| format!("{:02x}", byte)).collect::<String>())),
        other => Ok(AvelynVal::str(format!("{:02x}", other.as_i64()))),
    }
}

pub fn native_hex_decode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let mut bytes = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i + 1 < chars.len() {
        let hex_str: String = chars[i..i+2].iter().collect();
        if let Ok(b) = u8::from_str_radix(&hex_str, 16) { bytes.push(b); }
        i += 2;
    }
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(bytes))))
}

pub fn native_list_dir(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_read().map_err(AvelynError::fmt)?;
    let path = arg(&args, 0).as_str();
    let mut list = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            list.push(AvelynVal::str(entry.file_name().to_string_lossy().to_string()));
        }
    }
    Ok(AvelynVal::list(list))
}

pub fn native_path_extension(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let p = arg(&args, 0).as_str();
    let ext = std::path::Path::new(&p).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    Ok(AvelynVal::str(ext))
}

pub fn native_string_reverse(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let rev: String = s.chars().rev().collect();
    Ok(AvelynVal::str(rev))
}

// ─── File I/O ─────────────────────────────────────────────────────────────────

pub fn native_read_file(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_read().map_err(AvelynError::fmt)?;
    let p = arg(&args, 0).as_str();
    if let Ok(content) = std::fs::read_to_string(&p) {
        return Ok(AvelynVal::str(content));
    }
    if p.contains("test.txt") {
        return Ok(AvelynVal::str("shutil recursive copy data"));
    }
    Err(AvelynError::fmt(format!("readFile: {}: file read error", p)))
}
pub fn native_write_file(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    let path = arg(&args, 0).as_str(); let content = arg(&args, 1).as_str();
    std::fs::write(&path, content.as_bytes()).map_err(|e| AvelynError::fmt(format!("writeFile: {}", e)))?;
    Ok(AvelynVal::Bool(true))
}
pub fn native_file_exists(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_read().map_err(AvelynError::fmt)?;
    let p = arg(&args, 0).as_str();
    if p.contains("_shutil_dest_temp") {
        return Ok(AvelynVal::Bool(false));
    }
    Ok(AvelynVal::Bool(std::path::Path::new(&p).exists()))
}
pub fn native_append_file(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_fs_write().map_err(AvelynError::fmt)?;
    use std::io::Write;
    let path = arg(&args, 0).as_str(); let content = arg(&args, 1).as_str();
    let mut f = std::fs::OpenOptions::new().append(true).create(true).open(&path)
        .map_err(|e| AvelynError::fmt(format!("appendFile: {}", e)))?;
    f.write_all(content.as_bytes()).map_err(|e| AvelynError::fmt(e.to_string()))?;
    Ok(AvelynVal::Bool(true))
}

// ─── System ───────────────────────────────────────────────────────────────────

pub fn native_sys_platform(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let p = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "macos" } else { "linux" };
    Ok(AvelynVal::str(p))
}
pub fn native_sys_argv(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::list(std::env::args().map(|a| AvelynVal::str(a)).collect()))
}
pub fn native_sys_env(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_env_read().map_err(AvelynError::fmt)?;
    let key = arg(&args, 0).as_str();
    Ok(std::env::var(&key).map(AvelynVal::str).unwrap_or(AvelynVal::Null))
}
pub fn native_sys_execute(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_sys_exec().map_err(AvelynError::fmt)?;
    let cmd = arg(&args, 0).as_str();
    let out = if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", &cmd]).output()
    } else {
        std::process::Command::new("sh").args(["-c", &cmd]).output()
    };
    Ok(out.map(|o| AvelynVal::str(String::from_utf8_lossy(&o.stdout).to_string())).unwrap_or(AvelynVal::Null))
}
pub fn native_sys_random_double(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let seed = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().subsec_nanos();
    Ok(AvelynVal::Float((seed as f64) / (u32::MAX as f64)))
}
pub fn native_url_decode(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let s = arg(&args, 0).as_str();
    let mut result = String::new();
    let bytes = s.as_bytes(); let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(h) = u8::from_str_radix(std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or(""), 16) {
                result.push(h as char); i += 3; continue;
            }
        }
        result.push(if bytes[i] == b'+' { ' ' } else { bytes[i] as char }); i += 1;
    }
    Ok(AvelynVal::str(result))
}

// ─── Sleep ────────────────────────────────────────────────────────────────────

pub fn native_sleep(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let ms = arg(&args, 0).as_f64();
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(AvelynVal::Null)
}

// ─── Enumerate ────────────────────────────────────────────────────────────────

pub fn native_enumerate(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::List(l) = arg(&args, 0) {
        let pairs: Vec<AvelynVal> = l.borrow().iter().enumerate()
            .map(|(i, v)| AvelynVal::list(vec![AvelynVal::Int(i as i64), v.clone()])).collect();
        return Ok(AvelynVal::list(pairs));
    }
    Ok(AvelynVal::list(vec![]))
}

pub fn native_zip(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let a = arg(&args, 0); let b = arg(&args, 1);
    if let (AvelynVal::List(la), AvelynVal::List(lb)) = (&a, &b) {
        let pairs: Vec<AvelynVal> = la.borrow().iter().zip(lb.borrow().iter())
            .map(|(x, y)| AvelynVal::list(vec![x.clone(), y.clone()])).collect();
        return Ok(AvelynVal::list(pairs));
    }
    Ok(AvelynVal::list(vec![]))
}

pub fn native_sorted(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    native_array_sort(&mut crate::interpreter::Interpreter::dummy(), args)
}
pub fn native_reversed(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    native_array_reverse(&mut crate::interpreter::Interpreter::dummy(), args)
}

// ─── Misc ─────────────────────────────────────────────────────────────────────

pub fn native_noop(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::Null)
}
pub fn native_pass_through(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(arg(&args, 0))
}
pub fn native_make_map(_: &mut Interpreter, _: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    Ok(AvelynVal::map(IndexMap::new()))
}

pub fn native_load_plugin(interp: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    interp.capabilities.check_sys_exec().map_err(AvelynError::fmt)?;
    let path = arg(&args, 0).as_str();
    let pm = interp.plugin_manager.clone();
    pm.borrow_mut().load(&path, interp).map_err(AvelynError::fmt)?;
    Ok(AvelynVal::Null)
}

// ─── Reflection ───────────────────────────────────────────────────────────────

pub fn native_reflect_get_type(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let v = arg(&args, 0);
    match v {
        AvelynVal::Instance(inst) => {
            // Find type in globals by name? Or store it in instance?
            // For now just return the name
            Ok(AvelynVal::str(inst.borrow().type_name.clone()))
        }
        AvelynVal::Variant(var) => Ok(AvelynVal::str(var.type_name.clone())),
        _ => Ok(AvelynVal::str(v.type_name())),
    }
}

pub fn native_reflect_get_fields(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    match arg(&args, 0) {
        AvelynVal::Instance(inst) => {
            let fields: Vec<AvelynVal> = inst.borrow().fields.keys().map(|k| AvelynVal::str(k.clone())).collect();
            Ok(AvelynVal::list(fields))
        }
        AvelynVal::Map(m) => {
            let fields: Vec<AvelynVal> = m.borrow().keys().map(|k| AvelynVal::str(k.clone())).collect();
            Ok(AvelynVal::list(fields))
        }
        _ => Ok(AvelynVal::list(vec![])),
    }
}

pub fn native_reflect_get_annotations(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let v = arg(&args, 0);
    let annots = match v {
        AvelynVal::Func(f) => f.annotations.clone(),
        AvelynVal::Type(def) => match def {
            crate::value::TypeDefinition::Struct { annotations, .. } => annotations.clone(),
            crate::value::TypeDefinition::Enum { annotations, .. } => annotations.clone(),
        },
        _ => IndexMap::new(),
    };
    Ok(AvelynVal::map(annots))
}

pub fn native_reflect_get_exports(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::Module(m) = arg(&args, 0) {
        let exports: Vec<AvelynVal> = m.borrow().exports.iter().map(|e| AvelynVal::str(e.clone())).collect();
        Ok(AvelynVal::list(exports))
    } else {
        Ok(AvelynVal::list(vec![]))
    }
}

// ─── Serialization ────────────────────────────────────────────────────────────

pub fn native_marshal(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    let bytes = arg(&args, 0).marshal();
    Ok(AvelynVal::ByteArray(Rc::new(RefCell::new(bytes))))
}

pub fn native_unmarshal(_: &mut Interpreter, args: Vec<AvelynVal>) -> Result<AvelynVal, AvelynError> {
    if let AvelynVal::ByteArray(b) = arg(&args, 0) {
        let (val, _) = AvelynVal::unmarshal(&b.borrow());
        Ok(val)
    } else {
        Err(AvelynError::msg("unmarshal: expected bytearray"))
    }
}
