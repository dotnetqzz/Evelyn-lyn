// stdlib/io.rs — I/O and core builtins

use std::rc::Rc;
use crate::value::{SylError, SylIter, SylVal};
use crate::vm::Vm;
use std::cell::RefCell;

fn arg(args: &[SylVal], i: usize) -> SylVal {
    args.get(i).cloned().unwrap_or(SylVal::Null)
}

pub fn native_print(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    println!("{}", v.format());
    Ok(SylVal::Null)
}

pub fn native_print_no_newline(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    print!("{}", v.format());
    Ok(SylVal::Null)
}

pub fn native_time(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    Ok(SylVal::Float(ms as f64))
}

pub fn native_time_sec(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    Ok(SylVal::Float(secs as f64))
}

pub fn native_time_ms(_vm: &mut Vm, _args: &[SylVal]) -> Result<SylVal, SylError> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    Ok(SylVal::Float(ms as f64))
}

pub fn native_sleep(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let ms = arg(args, 0).as_i64();
    std::thread::sleep(std::time::Duration::from_millis(ms as u64));
    Ok(SylVal::Null)
}

pub fn native_to_string(vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    native_str(vm, args)
}

pub fn native_make_iter(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let src = arg(args, 0);
    let it = SylIter { source: src, pos: 0 };
    Ok(SylVal::Iter(Rc::new(RefCell::new(it))))
}

pub fn native_iter_next(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let iv = arg(args, 0);
    match &iv {
        SylVal::Iter(it) => Ok(it.borrow_mut().next_val()),
        _ => Ok(SylVal::Null),
    }
}

pub fn native_len(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    let n = match &v {
        SylVal::List(l) => l.borrow().len() as i64,
        SylVal::Str(s) => s.chars().count() as i64,
        SylVal::Map(m) => m.borrow().len() as i64,
        _ => 0,
    };
    Ok(SylVal::Int(n))
}

pub fn native_str(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    let radix_val = args.get(1);
    let radix = match radix_val {
        Some(SylVal::Int(r)) => *r as u32,
        _ => 10,
    };
    if radix != 10 && radix >= 2 && radix <= 36 {
        if let SylVal::Int(n) = &v {
            let n = *n;
            // i64::MIN-safe negation
            let is_neg = n < 0;
            let mut u: u64 = if is_neg { (n as i64).wrapping_neg() as u64 } else { n as u64 };
            let mut chars = Vec::new();
            if u == 0 { chars.push('0'); }
            while u > 0 {
                let rem = (u % radix as u64) as u8;
                chars.push(if rem < 10 { (b'0' + rem) as char } else { (b'a' + rem - 10) as char });
                u /= radix as u64;
            }
            if is_neg { chars.push('-'); }
            chars.reverse();
            return Ok(SylVal::Str(Rc::new(chars.into_iter().collect())));
        }
    }
    Ok(SylVal::Str(Rc::new(v.format())))
}

pub fn native_int_cast(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    let n = match &v {
        SylVal::Int(i) => *i,
        SylVal::Float(f) => *f as i64,
        SylVal::Bool(b) => if *b { 1 } else { 0 },
        SylVal::Str(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    };
    Ok(SylVal::Int(n))
}

pub fn native_float_cast(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    let f = match &v {
        SylVal::Int(i) => *i as f64,
        SylVal::Float(f) => *f,
        SylVal::Str(s) => s.parse::<f64>().unwrap_or(0.0),
        _ => 0.0,
    };
    Ok(SylVal::Float(f))
}

pub fn native_type_fn(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let v = arg(args, 0);
    Ok(SylVal::Str(Rc::new(v.type_name().to_string())))
}

pub fn native_exit(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let code = match arg(args, 0) { SylVal::Int(i) => i as i32, _ => 0 };
    std::process::exit(code);
}

pub fn native_assert(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let cond = arg(args, 0);
    let msg = args.get(1).map(|v| v.format()).unwrap_or_else(|| "Assertion failed".to_string());
    if !cond.is_truthy() {
        return Err(SylError::fmt(format!("AssertionError: {}", msg)));
    }
    Ok(SylVal::Null)
}

pub fn native_readline(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    if let Some(prompt) = args.get(0) {
        print!("{}", prompt.format());
        use std::io::Write;
        std::io::stdout().flush().ok();
    }
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    let line = line.trim_end_matches('\n').trim_end_matches('\r').to_string();
    Ok(SylVal::Str(Rc::new(line)))
}

pub fn native_input(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    native_readline(_vm, args)
}

pub fn native_range(_vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    let (start, end, step) = match args.len() {
        1 => (0i64, arg(args, 0).as_i64(), 1i64),
        2 => (arg(args, 0).as_i64(), arg(args, 1).as_i64(), 1i64),
        _ => (arg(args, 0).as_i64(), arg(args, 1).as_i64(), arg(args, 2).as_i64()),
    };
    let step = if step == 0 { 1 } else { step };
    let mut items = Vec::new();
    let mut i = start;
    while if step > 0 { i < end } else { i > end } {
        items.push(SylVal::Int(i));
        i += step;
    }
    Ok(SylVal::List(Rc::new(RefCell::new(items))))
}

pub fn native_call_with_args(vm: &mut Vm, args: &[SylVal]) -> Result<SylVal, SylError> {
    if args.len() < 2 { return Ok(SylVal::Null); }
    let callee = args[0].clone();
    let arg_list = match &args[1] {
        SylVal::List(l) => l.borrow().clone(),
        _ => return Ok(SylVal::Null),
    };
    match callee {
        SylVal::Native(f) => f(vm, &arg_list),
        SylVal::Func(_) => {
            // Func needs a module ref — not available here; push error
            Err(SylError::msg("__call_with_args__: cannot call Sylvel function without module context"))
        }
        _ => Ok(SylVal::Null),
    }
}

pub fn register(vm: &mut Vm) {
    vm.register_native("__print__",          native_print);
    vm.register_native("print",              native_print);
    vm.register_native("__print_no_nl__",    native_print_no_newline);
    vm.register_native("__time__",           native_time);
    vm.register_native("dateNow",            native_time);
    vm.register_native("timeSec",            native_time_sec);
    vm.register_native("timeMs",             native_time_ms);
    vm.register_native("timeSleep",           native_sleep);
    vm.register_native("toString",           native_to_string);
    vm.register_native("numToString",        native_to_string);
    vm.register_native("__make_iter__",      native_make_iter);
    vm.register_native("__iter_next__",      native_iter_next);
    vm.register_native("len",                native_len);
    vm.register_native("stringLen",          native_len);
    vm.register_native("str",                native_str);
    vm.register_native("int",                native_int_cast);
    vm.register_native("float",              native_float_cast);
    vm.register_native("type",               native_type_fn);
    vm.register_native("exit",               native_exit);
    vm.register_native("sysExit",            native_exit);
    vm.register_native("assert",             native_assert);
    vm.register_native("readline",           native_readline);
    vm.register_native("input",              native_input);
    vm.register_native("sysReadLine",        native_readline);
    vm.register_native("range",              native_range);
    vm.register_native("__call_with_args__", native_call_with_args);
}
