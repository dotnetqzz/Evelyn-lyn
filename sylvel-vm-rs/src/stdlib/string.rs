// stdlib/string.rs — String builtins

use std::rc::Rc;
use std::cell::RefCell;
use crate::value::{SylError, SylVal};
use crate::vm::Vm;

fn arg(args: &[SylVal], i: usize) -> SylVal {
    args.get(i).cloned().unwrap_or(SylVal::Null)
}

fn get_str(v: &SylVal) -> String { v.format() }

pub fn native_upper(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Str(Rc::new(get_str(&arg(args, 0)).to_uppercase())))
}

pub fn native_lower(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Str(Rc::new(get_str(&arg(args, 0)).to_lowercase())))
}

pub fn native_strip(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Str(Rc::new(get_str(&arg(args, 0)).trim().to_string())))
}

pub fn native_split(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let delim = get_str(&arg(args, 1));
    let parts: Vec<SylVal> = if delim.is_empty() {
        s.chars().map(|c| SylVal::Str(Rc::new(c.to_string()))).collect()
    } else {
        s.split(delim.as_str()).map(|p| SylVal::Str(Rc::new(p.to_string()))).collect()
    };
    Ok(SylVal::List(Rc::new(RefCell::new(parts))))
}

pub fn native_join(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let sep = get_str(&arg(args, 1));
    let list_val = arg(args, 0);
    let parts: Vec<String> = match &list_val {
        SylVal::List(l) => l.borrow().iter().map(|v| v.format()).collect(),
        _ => return Ok(SylVal::Str(Rc::new(String::new()))),
    };
    Ok(SylVal::Str(Rc::new(parts.join(&sep))))
}

pub fn native_replace(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let from = get_str(&arg(args, 1));
    let to = get_str(&arg(args, 2));
    Ok(SylVal::Str(Rc::new(s.replace(from.as_str(), &to))))
}

pub fn native_contains(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let needle = get_str(&arg(args, 1));
    Ok(SylVal::Bool(s.contains(needle.as_str())))
}

pub fn native_starts_with(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let prefix = get_str(&arg(args, 1));
    Ok(SylVal::Bool(s.starts_with(prefix.as_str())))
}

pub fn native_ends_with(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let suffix = get_str(&arg(args, 1));
    Ok(SylVal::Bool(s.ends_with(suffix.as_str())))
}

pub fn native_index_of(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let needle = get_str(&arg(args, 1));
    match s.find(needle.as_str()) {
        Some(i) => Ok(SylVal::Int(i as i64)),
        None => Ok(SylVal::Int(-1)),
    }
}

pub fn native_substring(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let start = arg(args, 1).as_i64() as usize;
    let end_val = arg(args, 2);
    let end = match &end_val {
        SylVal::Null => s.len(),
        _ => end_val.as_i64() as usize,
    };
    let start = start.min(s.len());
    let end = end.min(s.len());
    Ok(SylVal::Str(Rc::new(s[start..end].to_string())))
}

pub fn native_repeat(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let n = arg(args, 1).as_i64() as usize;
    Ok(SylVal::Str(Rc::new(s.repeat(n))))
}

pub fn native_char_code(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    Ok(SylVal::Int(s.chars().next().map(|c| c as i64).unwrap_or(0)))
}

pub fn native_char_from_code(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let code = arg(args, 0).as_i64() as u32;
    let c = char::from_u32(code).unwrap_or('\0');
    Ok(SylVal::Str(Rc::new(c.to_string())))
}

pub fn native_pad_start(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let len = arg(args, 1).as_i64() as usize;
    let pad = get_str(&arg(args, 2));
    let pad = if pad.is_empty() { " ".to_string() } else { pad };
    if s.len() >= len { return Ok(SylVal::Str(Rc::new(s))); }
    let needed = len - s.len();
    let filler = pad.repeat((needed / pad.len()) + 1);
    Ok(SylVal::Str(Rc::new(format!("{}{}", &filler[..needed], s))))
}

pub fn native_pad_end(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = get_str(&arg(args, 0));
    let len = arg(args, 1).as_i64() as usize;
    let pad = get_str(&arg(args, 2));
    let pad = if pad.is_empty() { " ".to_string() } else { pad };
    if s.len() >= len { return Ok(SylVal::Str(Rc::new(s))); }
    let needed = len - s.len();
    let filler = pad.repeat((needed / pad.len()) + 1);
    Ok(SylVal::Str(Rc::new(format!("{}{}", s, &filler[..needed]))))
}

pub fn register(vm: &mut Vm) {
    vm.register_native("upper",           native_upper);
    vm.register_native("lower",           native_lower);
    vm.register_native("strip",           native_strip);
    vm.register_native("trim",            native_strip);
    vm.register_native("split",           native_split);
    vm.register_native("join",            native_join);
    vm.register_native("replace",         native_replace);
    vm.register_native("contains",        native_contains);
    vm.register_native("startsWith",      native_starts_with);
    vm.register_native("endsWith",        native_ends_with);
    vm.register_native("indexOf",         native_index_of);
    vm.register_native("substring",       native_substring);
    vm.register_native("repeat",          native_repeat);
    vm.register_native("charCode",        native_char_code);
    vm.register_native("charFromCode",    native_char_from_code);
    vm.register_native("padStart",        native_pad_start);
    vm.register_native("padEnd",          native_pad_end);
    // string.* prefix
    vm.register_native("string.upper",    native_upper);
    vm.register_native("string.lower",    native_lower);
    vm.register_native("string.strip",    native_strip);
    vm.register_native("string.split",    native_split);
    vm.register_native("string.join",     native_join);
    vm.register_native("string.replace",  native_replace);
    vm.register_native("string.contains", native_contains);
    // stringXxx camelCase (used in .lyn tests)
    vm.register_native("stringUpper",     native_upper);
    vm.register_native("stringLower",     native_lower);
    vm.register_native("stringStrip",     native_strip);
    vm.register_native("stringTrim",      native_strip);
    vm.register_native("stringSplit",     native_split);
    vm.register_native("stringJoin",      native_join);
    vm.register_native("stringReplace",   native_replace);
    vm.register_native("stringContains",  native_contains);
    vm.register_native("stringStartsWith",native_starts_with);
    vm.register_native("stringEndsWith",  native_ends_with);
    vm.register_native("stringIndexOf",   native_index_of);
    vm.register_native("stringSubstring", native_substring);
    vm.register_native("stringRepeat",    native_repeat);
    vm.register_native("stringCharCode",  native_char_code);
    vm.register_native("stringFromCode",  native_char_from_code);
    vm.register_native("stringPadStart",  native_pad_start);
    vm.register_native("stringPadEnd",    native_pad_end);
    vm.register_native("stringFormat",    |_vm, args| {
        Ok(SylVal::Str(Rc::new(args.get(0).map(|v| v.format()).unwrap_or_default())))
    });
}
