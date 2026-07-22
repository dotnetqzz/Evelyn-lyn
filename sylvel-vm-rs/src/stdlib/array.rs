// stdlib/array.rs — Array and type-checking builtins

use std::rc::Rc;
use std::cell::RefCell;
use crate::value::{SylError, SylVal};
use crate::vm::Vm;

fn arg(args: &[SylVal], i: usize) -> SylVal {
    args.get(i).cloned().unwrap_or(SylVal::Null)
}

pub fn native_array_append(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let item = arg(args, 1);
    if let SylVal::List(l) = &arr {
        l.borrow_mut().push(item);
    }
    Ok(arr)
}

pub fn native_array_pop(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        return Ok(l.borrow_mut().pop().unwrap_or(SylVal::Null));
    }
    Ok(SylVal::Null)
}

pub fn native_array_get(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let idx = arg(args, 1).as_i64();
    if let SylVal::List(l) = &arr {
        let items = l.borrow();
        let len = items.len() as i64;
        let i = if idx < 0 { len + idx } else { idx };
        return Ok(if i >= 0 && (i as usize) < items.len() { items[i as usize].clone() } else { SylVal::Null });
    }
    Ok(SylVal::Null)
}

pub fn native_array_shift(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        let mut items = l.borrow_mut();
        if !items.is_empty() {
            return Ok(items.remove(0));
        }
    }
    Ok(SylVal::Null)
}

pub fn native_array_unshift(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let item = arg(args, 1);
    if let SylVal::List(l) = &arr {
        l.borrow_mut().insert(0, item);
    }
    Ok(arr)
}

pub fn native_array_insert(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let idx = arg(args, 1).as_i64() as usize;
    let item = arg(args, 2);
    if let SylVal::List(l) = &arr {
        let mut items = l.borrow_mut();
        let idx = idx.min(items.len());
        items.insert(idx, item);
    }
    Ok(arr)
}

pub fn native_array_remove(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let idx = arg(args, 1).as_i64();
    if let SylVal::List(l) = &arr {
        let mut items = l.borrow_mut();
        let len = items.len() as i64;
        let i = if idx < 0 { len + idx } else { idx };
        if i >= 0 && (i as usize) < items.len() {
            return Ok(items.remove(i as usize));
        }
    }
    Ok(SylVal::Null)
}

pub fn native_array_slice(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        let items = l.borrow();
        let len = items.len() as i64;
        let start_raw = arg(args, 1).as_i64();
        let start = (if start_raw < 0 { len + start_raw } else { start_raw }).max(0) as usize;
        let end = match args.get(2) {
            Some(v) => {
                let e = v.as_i64();
                ((if e < 0 { len + e } else { e }).max(0) as usize).min(items.len())
            }
            None => items.len(),
        };
        let start = start.min(items.len());
        let end = end.min(items.len()).max(start); // start <= end always
        let sliced = items[start..end].to_vec();
        return Ok(SylVal::List(Rc::new(RefCell::new(sliced))));
    }
    Ok(SylVal::Null)
}

pub fn native_array_concat(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let a = arg(args, 0);
    let b = arg(args, 1);
    if let (SylVal::List(la), SylVal::List(lb)) = (&a, &b) {
        let mut combined = la.borrow().clone();
        combined.extend(lb.borrow().iter().cloned());
        return Ok(SylVal::List(Rc::new(RefCell::new(combined))));
    }
    Ok(SylVal::Null)
}

pub fn native_array_reverse(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        let mut items = l.borrow().clone();
        items.reverse();
        return Ok(SylVal::List(Rc::new(RefCell::new(items))));
    }
    Ok(arr)
}

pub fn native_array_sort(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        let mut items = l.borrow().clone();
        items.sort_by(|a, b| {
            match (a, b) {
                (SylVal::Int(x), SylVal::Int(y)) => x.cmp(y),
                (SylVal::Float(x), SylVal::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
                (SylVal::Str(x), SylVal::Str(y)) => x.cmp(y),
                (a, b) => a.format().cmp(&b.format()),
            }
        });
        return Ok(SylVal::List(Rc::new(RefCell::new(items))));
    }
    Ok(arr)
}

pub fn native_array_contains(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let item = arg(args, 1);
    if let SylVal::List(l) = &arr {
        return Ok(SylVal::Bool(l.borrow().iter().any(|v| v.deep_equal(&item))));
    }
    Ok(SylVal::Bool(false))
}

pub fn native_array_index_of(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    let item = arg(args, 1);
    if let SylVal::List(l) = &arr {
        let idx = l.borrow().iter().position(|v| v.deep_equal(&item));
        return Ok(SylVal::Int(idx.map(|i| i as i64).unwrap_or(-1)));
    }
    Ok(SylVal::Int(-1))
}

pub fn native_array_copy(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        return Ok(SylVal::List(Rc::new(RefCell::new(l.borrow().clone()))));
    }
    Ok(arr)
}

pub fn native_array_flatten(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    fn flatten(v: &SylVal, out: &mut Vec<SylVal>) {
        if let SylVal::List(l) = v {
            for item in l.borrow().iter() { flatten(item, out); }
        } else {
            out.push(v.clone());
        }
    }
    let arr = arg(args, 0);
    let mut out = Vec::new();
    flatten(&arr, &mut out);
    Ok(SylVal::List(Rc::new(RefCell::new(out))))
}

pub fn native_array_unique(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let arr = arg(args, 0);
    if let SylVal::List(l) = &arr {
        let mut seen: Vec<SylVal> = Vec::new();
        for item in l.borrow().iter() {
            if !seen.iter().any(|s| s.deep_equal(item)) {
                seen.push(item.clone());
            }
        }
        return Ok(SylVal::List(Rc::new(RefCell::new(seen))));
    }
    Ok(arr)
}

// ── Type checks ────────────────────────────────────────────────────────────

pub fn native_is_null(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Null)))
}
pub fn native_is_string(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Str(_))))
}
pub fn native_is_number(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Int(_) | SylVal::Float(_))))
}
pub fn native_is_bool(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Bool(_))))
}
pub fn native_is_array(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::List(_))))
}
pub fn native_is_map(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Map(_))))
}
pub fn native_is_function(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Func(_) | SylVal::Native(_))))
}
pub fn native_is_integer(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(matches!(arg(args, 0), SylVal::Int(_))))
}

// ── Conversions ────────────────────────────────────────────────────────────

pub fn native_to_number(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    match arg(args, 0) {
        SylVal::Int(i) => Ok(SylVal::Int(i)),
        SylVal::Float(f) => Ok(SylVal::Float(f)),
        SylVal::Bool(b) => Ok(SylVal::Int(if b { 1 } else { 0 })),
        SylVal::Str(s) => {
            if let Ok(i) = s.parse::<i64>() { return Ok(SylVal::Int(i)); }
            if let Ok(f) = s.parse::<f64>() { return Ok(SylVal::Float(f)); }
            Ok(SylVal::Null)
        }
        _ => Ok(SylVal::Null),
    }
}

pub fn native_to_bool(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(arg(args, 0).is_truthy()))
}

pub fn native_to_array(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    match arg(args, 0) {
        SylVal::List(l) => Ok(SylVal::List(l)),
        v => Ok(SylVal::List(Rc::new(RefCell::new(vec![v])))),
    }
}

pub fn native_deep_copy(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    fn copy(v: &SylVal) -> SylVal {
        match v {
            SylVal::List(l) => SylVal::List(Rc::new(RefCell::new(l.borrow().iter().map(copy).collect()))),
            SylVal::Map(m) => {
                use indexmap::IndexMap;
                let new_map: IndexMap<String, SylVal> = m.borrow().iter().map(|(k, v)| (k.clone(), copy(v))).collect();
                SylVal::Map(Rc::new(RefCell::new(new_map)))
            }
            other => other.clone(),
        }
    }
    Ok(copy(&arg(args, 0)))
}

pub fn native_deep_equal(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    Ok(SylVal::Bool(arg(args, 0).deep_equal(&arg(args, 1))))
}

pub fn native_hash(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let s = arg(args, 0).format();
    let mut h = 2166136261u32;
    for b in s.bytes() { h ^= b as u32; h = h.wrapping_mul(16777619); }
    Ok(SylVal::Float((h % 1000000007) as f64))
}

pub fn register(vm: &mut Vm) {
    // arrayXxx
    vm.register_native("arrayGet",      native_array_get);
    vm.register_native("getAtIndex",    native_array_get);
    vm.register_native("arrayLen",      crate::stdlib::io::native_len);
    vm.register_native("arrayAppend",   native_array_append);
    vm.register_native("arrayPush",     native_array_append);
    vm.register_native("arrayPop",      native_array_pop);
    vm.register_native("arrayShift",    native_array_shift);
    vm.register_native("arrayUnshift",  native_array_unshift);
    vm.register_native("arrayInsert",   native_array_insert);
    vm.register_native("arrayRemove",   native_array_remove);
    vm.register_native("arraySlice",    native_array_slice);
    vm.register_native("arrayConcat",   native_array_concat);
    vm.register_native("arrayReverse",  native_array_reverse);
    vm.register_native("arraySort",     native_array_sort);
    vm.register_native("arrayContains", native_array_contains);
    vm.register_native("arrayIndexOf",  native_array_index_of);
    vm.register_native("arrayCopy",     native_array_copy);
    vm.register_native("arrayFlatten",  native_array_flatten);
    vm.register_native("arrayUnique",   native_array_unique);
    vm.register_native("contains",      native_array_contains);
    // Type checks
    vm.register_native("isNull",        native_is_null);
    vm.register_native("isString",      native_is_string);
    vm.register_native("isNumber",      native_is_number);
    vm.register_native("isBool",        native_is_bool);
    vm.register_native("isArray",       native_is_array);
    vm.register_native("isMap",         native_is_map);
    vm.register_native("isFunction",    native_is_function);
    vm.register_native("isInteger",     native_is_integer);
    // Conversions
    vm.register_native("toNumber",      native_to_number);
    vm.register_native("toBool",        native_to_bool);
    vm.register_native("toArray",       native_to_array);
    vm.register_native("stringToNum",   native_to_number);
    vm.register_native("toNum",         native_to_number);
    vm.register_native("parseNum",      native_to_number);
    vm.register_native("deepCopy",      native_deep_copy);
    vm.register_native("deepEqual",     native_deep_equal);
    vm.register_native("hash",          native_hash);
    vm.register_native("numToString",   crate::stdlib::io::native_str);
    vm.register_native("toString",      crate::stdlib::io::native_str);
}
