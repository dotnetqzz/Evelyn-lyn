// interpreter/mod.rs — Tree-walking interpreter engine
// Ported from CoreInterpreter/SysLib.swift

pub mod builtins;
pub mod module_manager;
pub mod capabilities;
pub mod plugin;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use indexmap::IndexMap;

use crate::ast::ASTNode;
use crate::env::Env;
use crate::value::{AvelynError, AvelynFunc, AvelynVal, NativeFn, Signal};
use self::module_manager::{ModuleManager, ModuleSource};
use self::capabilities::Capabilities;
use self::plugin::PluginManager;

pub struct Interpreter {
    pub globals: Rc<Env>,
    pub current_file: String,
    pub current_line: u32,
    pub call_stack: Vec<(String, String, u32)>, // (func_name, file, line)
    pub native_registry: HashMap<String, NativeFn>,
    pub module_manager: ModuleManager,
    pub module_cache: HashMap<String, AvelynVal>,
    pub current_module: Option<Rc<RefCell<crate::value::Module>>>,
    pub capabilities: Capabilities,
    pub plugin_manager: Rc<RefCell<PluginManager>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Env::new();
        let mut interp = Interpreter {
            globals,
            current_file: "<main>".into(),
            current_line: 1,
            call_stack: Vec::new(),
            native_registry: HashMap::new(),
            module_manager: ModuleManager::new(),
            module_cache: HashMap::new(),
            current_module: None,
            capabilities: Capabilities::all(),
            plugin_manager: Rc::new(RefCell::new(PluginManager::new())),
        };
        interp.register_builtins();
        interp
    }

    pub fn dummy() -> Self {
        Interpreter {
            globals: Env::new(),
            current_file: "".into(),
            current_line: 0,
            call_stack: Vec::new(),
            native_registry: HashMap::new(),
            module_manager: ModuleManager::new(),
            module_cache: HashMap::new(),
            current_module: None,
            capabilities: Capabilities::none(),
            plugin_manager: Rc::new(RefCell::new(PluginManager::new())),
        }
    }

    fn register_builtins(&mut self) {
        macro_rules! reg {
            ($name:expr, $func:path) => {
                self.globals.declare($name, AvelynVal::Native($func));
                self.native_registry.insert($name.to_string(), $func);
            };
        }

        // IO
        reg!("print", builtins::native_print);
        reg!("printNoNl", builtins::native_print_no_nl);
        reg!("input", builtins::native_input);
        reg!("time", builtins::native_time);
        reg!("exit", builtins::native_exit);

        // Types
        reg!("type", builtins::native_type);
        reg!("str", builtins::native_str);
        reg!("int", builtins::native_int);
        reg!("float", builtins::native_float);
        reg!("bool", builtins::native_bool);
        reg!("isNull", builtins::native_is_null);
        reg!("isString", builtins::native_is_string);
        reg!("isNumber", builtins::native_is_number);
        reg!("isBool", builtins::native_is_bool);
        reg!("isArray", builtins::native_is_array);
        reg!("isMap", builtins::native_is_map);
        reg!("isFunction", builtins::native_is_function);
        reg!("isInteger", builtins::native_is_integer);
        reg!("toNumber", builtins::native_to_number);

        // Collections & Math
        reg!("len", builtins::native_len);
        reg!("range", builtins::native_range);
        reg!("assert", builtins::native_assert);
        reg!("abs", builtins::native_abs);
        reg!("sqrt", builtins::native_sqrt);
        reg!("floor", builtins::native_floor);
        reg!("ceil", builtins::native_ceil);
        reg!("round", builtins::native_round);
        reg!("sin", builtins::native_sin);
        reg!("cos", builtins::native_cos);
        reg!("tan", builtins::native_tan);
        reg!("pow", builtins::native_pow);
        reg!("log", builtins::native_log);
        reg!("log2", builtins::native_log2);
        reg!("log10", builtins::native_log10);
        reg!("exp", builtins::native_exp);
        reg!("min", builtins::native_min);
        reg!("max", builtins::native_max);
        reg!("clamp", builtins::native_clamp);

        // String
        reg!("upper", builtins::native_upper);
        reg!("lower", builtins::native_lower);
        reg!("strip", builtins::native_strip);
        reg!("split", builtins::native_split);
        reg!("join", builtins::native_join);
        reg!("replace", builtins::native_replace);
        reg!("contains", builtins::native_contains);
        reg!("startsWith", builtins::native_starts_with);
        reg!("endsWith", builtins::native_ends_with);
        reg!("indexOf", builtins::native_index_of);
        reg!("substring", builtins::native_substring);
        reg!("repeat", builtins::native_repeat);
        reg!("charCodeAt", builtins::native_char_code);
        reg!("charFromCode", builtins::native_char_from_code);
        reg!("stringLen", builtins::native_string_len);

        // Arrays
        reg!("arrayAppend", builtins::native_array_append);
        reg!("arrayPop", builtins::native_array_pop);
        reg!("arrayShift", builtins::native_array_shift);
        reg!("arrayUnshift", builtins::native_array_unshift);
        reg!("arrayInsert", builtins::native_array_insert);
        reg!("arrayRemove", builtins::native_array_remove);
        reg!("arrayGet", builtins::native_array_get);
        reg!("arraySlice", builtins::native_array_slice);
        reg!("arrayConcat", builtins::native_array_concat);
        reg!("arrayReverse", builtins::native_array_reverse);
        reg!("arraySort", builtins::native_array_sort);
        reg!("arrayContains", builtins::native_array_contains);
        reg!("arrayIndexOf", builtins::native_array_index_of);
        reg!("arrayCopy", builtins::native_array_copy);
        reg!("arrayFlatten", builtins::native_array_flatten);
        reg!("arrayUnique", builtins::native_array_unique);

        // Map
        reg!("keys", builtins::native_keys);
        reg!("values", builtins::native_values);
        reg!("items", builtins::native_items);
        reg!("mapSet", builtins::native_map_set);
        reg!("mapGet", builtins::native_map_get);
        reg!("mapDelete", builtins::native_map_delete);
        reg!("mapHas", builtins::native_map_has);

        // Utils
        reg!("deepCopy", builtins::native_deep_copy);
        reg!("deepEqual", builtins::native_deep_equal);
        reg!("jsonEncode", builtins::native_json_encode);
        reg!("jsonDecode", builtins::native_json_decode);
        reg!("hashMd5", builtins::native_hash_md5);
        reg!("hashSha256", builtins::native_hash_sha256);

        // Files
        reg!("readFile", builtins::native_read_file);
        reg!("writeFile", builtins::native_write_file);
        reg!("fileExists", builtins::native_file_exists);
        reg!("appendFile", builtins::native_append_file);

        // System
        reg!("sysPlatform", builtins::native_sys_platform);
        reg!("sysArgv", builtins::native_sys_argv);
        reg!("sysEnv", builtins::native_sys_env);
        reg!("sysExecute", builtins::native_sys_execute);
        reg!("sysRandomDouble", builtins::native_sys_random_double);
        reg!("urlEncode", builtins::native_url_encode);
        reg!("urlDecode", builtins::native_url_decode);
        reg!("sleep", builtins::native_sleep);

        // Iterable
        reg!("enumerate", builtins::native_enumerate);
        reg!("zip", builtins::native_zip);
        reg!("sorted", builtins::native_sorted);
        reg!("reversed", builtins::native_reversed);

        // Aliases / stdlib helpers
        reg!("arrayLen", builtins::native_len);
        reg!("toString", builtins::native_str);
        reg!("numToString", builtins::native_str);
        reg!("stringContains", builtins::native_contains);
        reg!("stringStartsWith", builtins::native_starts_with);
        reg!("stringEndsWith", builtins::native_ends_with);
        reg!("stringSub", builtins::native_substring);
        reg!("stringReverse", builtins::native_string_reverse);
        reg!("stringSplit", builtins::native_split);
        reg!("stringJoin", builtins::native_join);
        reg!("stringReplace", builtins::native_replace);
        reg!("stringReplaceAll", builtins::native_replace);
        reg!("stringUpper", builtins::native_upper);
        reg!("stringToUpper", builtins::native_upper);
        reg!("stringLower", builtins::native_lower);
        reg!("stringToLower", builtins::native_lower);
        reg!("stringStrip", builtins::native_strip);
        reg!("fileRead", builtins::native_read_file);
        reg!("copyTree", builtins::native_copy_tree);
        reg!("rmTree", builtins::native_remove_dir_all);
        reg!("dirRemove", builtins::native_remove_dir_all);
        reg!("fileWrite", builtins::native_write_file);
        reg!("mathSqrt", builtins::native_sqrt);
        reg!("mathFloor", builtins::native_floor);
        reg!("mathCeil", builtins::native_ceil);
        reg!("mathRound", builtins::native_round);
        reg!("mathSin", builtins::native_sin);
        reg!("mathCos", builtins::native_cos);
        reg!("mathTan", builtins::native_tan);
        reg!("mathPow", builtins::native_pow);
        reg!("mathLog", builtins::native_log);
        reg!("mathLog2", builtins::native_log2);
        reg!("mathLog10", builtins::native_log10);
        reg!("mathExp", builtins::native_exp);
        reg!("mathAbs", builtins::native_abs);
        reg!("mathMin", builtins::native_min);
        reg!("mathMax", builtins::native_max);
        reg!("numToString", builtins::native_str);
        reg!("toString", builtins::native_str);
        reg!("stringContains", builtins::native_contains);
        reg!("base64Encode", builtins::native_base64_encode);
        reg!("base64Decode", builtins::native_base64_decode);
        reg!("dirExists", builtins::native_file_exists);
        reg!("pathDirname", builtins::native_path_dirname);
        reg!("randomInt", builtins::native_sys_random_int);
        reg!("dirCreate", builtins::native_dir_create);
        reg!("stringConcat", builtins::native_string_concat);
        reg!("pathJoin", builtins::native_path_join);
        reg!("arrayPush", builtins::native_array_append);
        reg!("randomBytes", builtins::native_sys_random_bytes);
        reg!("hexEncode", builtins::native_hex_encode);
        reg!("hexDecode", builtins::native_hex_decode);
        reg!("dirList", builtins::native_list_dir);
        reg!("pathExtension", builtins::native_path_extension);
        reg!("dateNow", builtins::native_time);
        reg!("timeSleep", builtins::native_sleep);
        reg!("stringIndexOf", builtins::native_index_of);
        reg!("stringAt", builtins::native_string_at);
        reg!("stringTrim", builtins::native_strip);
        reg!("mapKeys", builtins::native_keys);
        reg!("mapValues", builtins::native_values);
        reg!("timeSec", builtins::native_time_sec);
        reg!("dateFormat", builtins::native_date_format);
        reg!("pathBasename", builtins::native_path_basename);
        reg!("sysArch", builtins::native_sys_arch);
        reg!("sysRemoveFile", builtins::native_sys_remove_file);
        reg!("sysSecureRandomBytes", builtins::native_sys_random_bytes);
        reg!("sysUrlParse", builtins::native_sys_url_parse);
        reg!("uuidV4", builtins::native_uuid_v4);
        reg!("sysRegexGroups", builtins::native_sys_regex_groups);
        reg!("numCpus", builtins::native_num_cpus);
        reg!("spawnWorkers", builtins::native_spawn_workers);
        reg!("getAtIndex", builtins::native_array_get);
        reg!("sha512", builtins::native_sha512);
        reg!("netDnsLookup", builtins::native_net_dns_lookup);
        reg!("httpRequest", builtins::native_http_request);
        reg!("jsonStringify", builtins::native_json_encode);
        reg!("httpDirBrute", builtins::native_http_dir_brute);
        reg!("netSendTo", builtins::native_net_send_to);
        reg!("netListen", builtins::native_net_tcp_listen);
        reg!("netSetNonBlocking", builtins::native_noop);
        reg!("netConnect", builtins::native_net_tcp_connect);
        reg!("netAccept", builtins::native_net_accept);
        reg!("netSend", builtins::native_net_send);
        reg!("netRecv", builtins::native_net_recv);
        reg!("netPortScan", builtins::native_net_port_scan);
        reg!("netClose", builtins::native_noop);
        reg!("netRecvFrom", builtins::native_net_recv_from);
        reg!("netUdpBind", builtins::native_net_udp_socket);
        reg!("netUdpSocket", builtins::native_net_udp_socket);
        reg!("aesEncrypt", builtins::native_aes_encrypt);
        reg!("aesDecrypt", builtins::native_aes_decrypt);
        reg!("hmac", builtins::native_hmac);
        reg!("urlEncode", builtins::native_url_encode);
        reg!("sysLastErrorTraceback", builtins::native_sys_last_error_traceback);
        reg!("jsonParse", builtins::native_json_decode);
        reg!("timeMs", builtins::native_time);
        reg!("stringToNum", builtins::native_to_number);
        reg!("stringSplitLines", builtins::native_string_split_lines);
        reg!("sysSecureRandomDouble", builtins::native_sys_random_double);
        reg!("sysRegexGroups", builtins::native_sys_regex_groups);
        reg!("sysRegexFindAll", builtins::native_sys_regex_groups);
        reg!("sysRegexMatch", builtins::native_sys_regex_match);
        reg!("sysRegexReplace", builtins::native_sys_regex_sub);
        reg!("reSub", builtins::native_sys_regex_sub);
        reg!("sha1", builtins::native_sha1);
        reg!("sha256", builtins::native_hash_sha256);
        reg!("md5", builtins::native_hash_md5);
        reg!("makeMap", builtins::native_make_map);
        reg!("loadPlugin", builtins::native_load_plugin);

        // Reflection
        reg!("reflectGetType", builtins::native_reflect_get_type);
        reg!("reflectGetFields", builtins::native_reflect_get_fields);
        reg!("reflectGetAnnotations", builtins::native_reflect_get_annotations);
        reg!("reflectGetExports", builtins::native_reflect_get_exports);

        // Serialization
        reg!("marshal", builtins::native_marshal);
        reg!("unmarshal", builtins::native_unmarshal);
    }

    pub fn eval_ast(&mut self, ast: &[ASTNode]) -> Result<AvelynVal, AvelynError> {
        self.eval_ast_with_env(ast, &self.globals.clone())
    }

    pub fn eval_ast_with_env(&mut self, ast: &[ASTNode], env: &Rc<Env>) -> Result<AvelynVal, AvelynError> {
        let mut last = AvelynVal::Null;
        for node in ast {
            match self.eval_node(node, env) {
                Ok(v) => last = v,
                Err(Signal::Return(v)) => return Ok(v),
                Err(Signal::Error(e)) => return Err(e),
                Err(Signal::Break) => return Err(AvelynError::msg("SyntaxError: break outside loop")),
                Err(Signal::Continue) => return Err(AvelynError::msg("SyntaxError: continue outside loop")),
            }
        }
        Ok(last)
    }

    pub fn eval_node(&mut self, node: &ASTNode, env: &Rc<Env>) -> Result<AvelynVal, Signal> {
        match node {
            ASTNode::Int(i) => Ok(AvelynVal::Int(*i)),
            ASTNode::Float(f) => Ok(AvelynVal::Float(*f)),
            ASTNode::Str(s) => Ok(AvelynVal::str(s.clone())),
            ASTNode::Bool(b) => Ok(AvelynVal::Bool(*b)),
            ASTNode::Null => Ok(AvelynVal::Null),
            ASTNode::ByteArray(b) => Ok(AvelynVal::ByteArray(Rc::new(std::cell::RefCell::new(b.clone())))),

            ASTNode::Var(name) => {
                if let Some(v) = env.get(name) { Ok(v) }
                else { Err(Signal::Error(AvelynError::fmt(format!("NameError: variable '{}' is not defined", name)))) }
            }

            ASTNode::Decl { name, value, mutable: _, annotations } => {
                let val = self.eval_node(value, env)?;
                let mut annots = IndexMap::new();
                for a in annotations {
                    let a_val = self.eval_node(a, env)?;
                    annots.insert(a.to_string_key(), a_val);
                }
                env.declare(name, val);
                Ok(AvelynVal::Null)
            }

            ASTNode::StructDecl { name, fields, annotations } => {
                let mut annots = IndexMap::new();
                for a in annotations {
                    let a_val = self.eval_node(a, env)?;
                    annots.insert(a.to_string_key(), a_val);
                }
                let def = crate::value::TypeDefinition::Struct {
                    name: name.clone(),
                    fields: fields.clone(),
                    annotations: annots,
                };
                env.declare(name, AvelynVal::Type(def));
                Ok(AvelynVal::Null)
            }

            ASTNode::EnumDecl { name, variants, annotations } => {
                let mut annots = IndexMap::new();
                for a in annotations {
                    let a_val = self.eval_node(a, env)?;
                    annots.insert(a.to_string_key(), a_val);
                }
                let mut v_map = HashMap::new();
                for v in variants {
                    v_map.insert(v.name.clone(), (v.arity, v.fields.clone()));
                }
                let def = crate::value::TypeDefinition::Enum {
                    name: name.clone(),
                    variants: v_map,
                    annotations: annots,
                };
                env.declare(name, AvelynVal::Type(def));
                Ok(AvelynVal::Null)
            }

            ASTNode::Assign { name, value } => {
                let val = self.eval_node(value, env)?;
                env.set(name, val);
                Ok(AvelynVal::Null)
            }

            ASTNode::CompoundAssign { name, op, value } => {
                let rhs = self.eval_node(value, env)?;
                let lhs = env.get(name).unwrap_or(AvelynVal::Null);
                let res = self.eval_bin_op(&lhs, op, &rhs)?;
                env.set(name, res);
                Ok(AvelynVal::Null)
            }

            ASTNode::IndexAssign { target, index, value } => {
                let idx_val = self.eval_node(index, env)?;
                let val = self.eval_node(value, env)?;
                if let Some(t_val) = env.get(target) {
                    match t_val {
                        AvelynVal::List(l) => {
                            let raw = idx_val.as_i64();
                            let mut vec = l.borrow_mut();
                            let len = vec.len() as i64;
                            let idx = if raw < 0 { len + raw } else { raw };
                            if idx >= 0 {
                                let idx = idx as usize;
                                if idx < vec.len() { vec[idx] = val; }
                                else if idx == vec.len() { vec.push(val); }
                            }
                        }
                        AvelynVal::Map(m) => {
                            m.borrow_mut().insert(idx_val.as_str(), val);
                        }
                        _ => {}
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::DestructureArray { names, value, mutable: _ } => {
                let val = self.eval_node(value, env)?;
                if let AvelynVal::List(l) = val {
                    let vec = l.borrow();
                    for (i, name_opt) in names.iter().enumerate() {
                        if let Some(name) = name_opt {
                            let elem = vec.get(i).cloned().unwrap_or(AvelynVal::Null);
                            env.declare(name, elem);
                        }
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::DestructureMap { keys, value, mutable: _ } => {
                let val = self.eval_node(value, env)?;
                if let AvelynVal::Map(m) = val {
                    let map = m.borrow();
                    for (k, alias_opt) in keys {
                        let target_name = alias_opt.as_ref().unwrap_or(k);
                        let elem = map.get(k).cloned().unwrap_or(AvelynVal::Null);
                        env.declare(target_name, elem);
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::ArrayLit(elements) => {
                let mut list = Vec::new();
                for elem in elements {
                    if let ASTNode::Spread(inner) = elem {
                        let spread_val = self.eval_node(inner, env)?;
                        if let AvelynVal::List(l) = spread_val {
                            list.extend(l.borrow().iter().cloned());
                        }
                    } else {
                        list.push(self.eval_node(elem, env)?);
                    }
                }
                Ok(AvelynVal::list(list))
            }

            ASTNode::MapLit(pairs) => {
                let mut map = IndexMap::new();
                for (k_node, v_node) in pairs {
                    let k = self.eval_node(k_node, env)?.as_str();
                    let v = self.eval_node(v_node, env)?;
                    map.insert(k, v);
                }
                Ok(AvelynVal::map(map))
            }

            ASTNode::InterpStr(parts) => {
                let mut s = String::new();
                for part in parts {
                    let v = self.eval_node(part, env)?;
                    s.push_str(&v.format());
                }
                Ok(AvelynVal::str(s))
            }

            ASTNode::BinOp { left, op, right } => {
                // Short-circuit logical ops
                if op == "&&" || op == "and" {
                    let l = self.eval_node(left, env)?;
                    if !l.is_truthy() { return Ok(l); }
                    return self.eval_node(right, env);
                }
                if op == "||" || op == "or" {
                    let l = self.eval_node(left, env)?;
                    if l.is_truthy() { return Ok(l); }
                    return self.eval_node(right, env);
                }
                let l = self.eval_node(left, env)?;
                let r = self.eval_node(right, env)?;
                self.eval_bin_op(&l, op, &r)
            }

            ASTNode::UnaryOp { op, operand } => {
                let val = self.eval_node(operand, env)?;
                match op.as_str() {
                    "-" => Ok(match val {
                        AvelynVal::Int(i) => AvelynVal::Int(-i),
                        AvelynVal::Float(f) => AvelynVal::Float(-f),
                        _ => AvelynVal::Null,
                    }),
                    "!" | "not" => Ok(AvelynVal::Bool(!val.is_truthy())),
                    "~" => Ok(AvelynVal::Int(!val.as_i64())),
                    _ => Ok(AvelynVal::Null),
                }
            }

            ASTNode::Subscript { target, index } => {
                let t_val = self.eval_node(target, env)?;
                let i_val = self.eval_node(index, env)?;
                match t_val {
                    AvelynVal::List(l) => {
                        let vec = l.borrow();
                        let idx = i_val.as_i64();
                        let len = vec.len() as i64;
                        let pos = if idx < 0 { len + idx } else { idx } as usize;
                        Ok(vec.get(pos).cloned().unwrap_or(AvelynVal::Null))
                    }
                    AvelynVal::ByteArray(b) => {
                        let vec = b.borrow();
                        let idx = i_val.as_i64();
                        let len = vec.len() as i64;
                        let pos = if idx < 0 { len + idx } else { idx } as usize;
                        Ok(vec.get(pos).map(|byte| AvelynVal::Int(*byte as i64)).unwrap_or(AvelynVal::Null))
                    }
                    AvelynVal::Map(m) => {
                        let map = m.borrow();
                        Ok(map.get(&i_val.as_str()).cloned().unwrap_or(AvelynVal::Null))
                    }
                    AvelynVal::Instance(inst) => {
                        let inst = inst.borrow();
                        Ok(inst.fields.get(&i_val.as_str()).cloned().unwrap_or(AvelynVal::Null))
                    }
                    AvelynVal::Module(m) => {
                        let m = m.borrow();
                        let key = i_val.as_str();
                        if m.exports.contains(&key) {
                            Ok(m.env.get(&key).unwrap_or(AvelynVal::Null))
                        } else {
                            Err(Signal::Error(AvelynError::fmt(format!("NameError: module '{}' does not export '{}'", m.name, key))))
                        }
                    }
                    AvelynVal::Str(s) => {
                        let idx = i_val.as_i64();
                        let chars: Vec<char> = s.chars().collect();
                        let len = chars.len() as i64;
                        let pos = if idx < 0 { len + idx } else { idx };
                        if pos >= 0 {
                            Ok(chars.get(pos as usize).map(|c| AvelynVal::str(c.to_string())).unwrap_or(AvelynVal::Null))
                        } else { Ok(AvelynVal::Null) }
                    }
                    AvelynVal::Type(def) => match def {
                        crate::value::TypeDefinition::Enum { name, variants, .. } => {
                            let v_name = i_val.as_str();
                            if let Some((arity, _)) = variants.get(&v_name) {
                                if *arity == 0 {
                                    Ok(AvelynVal::Variant(Rc::new(crate::value::EnumVariantInstance {
                                        type_name: name.clone(),
                                        variant_name: v_name,
                                        values: vec![],
                                    })))
                                } else {
                                    // Return a constructor function using ASTNode::Lambda for closure support
                                    let t_name = name.clone();
                                    let v_name_inner = v_name.clone();

                                    // Alternative: Define a specialized NativeFn that takes the info from somewhere?
                                    // No, let's just use AvelynVal::Func with a custom "body" that we can identify.
                                    // Actually, let's just make EnumVariant a first-class callable in eval_call.

                                    Ok(AvelynVal::Variant(Rc::new(crate::value::EnumVariantInstance {
                                        type_name: t_name,
                                        variant_name: v_name_inner,
                                        values: vec![], // Partially applied? No, Avelyn doesn't support that easily.
                                    })))
                                }
                            } else {
                                Err(Signal::Error(AvelynError::fmt(format!("NameError: enum '{}' has no variant '{}'", name, v_name))))
                            }
                        }
                        _ => Ok(AvelynVal::Null),
                    }
                    _ => Ok(AvelynVal::Null),
                }
            }

            ASTNode::Ternary { cond, then, els } => {
                let c = self.eval_node(cond, env)?;
                if c.is_truthy() { self.eval_node(then, env) } else { self.eval_node(els, env) }
            }

            ASTNode::NullCoalesce { left, right } => {
                let l = self.eval_node(left, env)?;
                if !l.is_null() { Ok(l) } else { self.eval_node(right, env) }
            }

            ASTNode::Spread(_) | ASTNode::NamedArg { .. } => Ok(AvelynVal::Null),

            ASTNode::FuncDecl { name, params, body, variadic, annotations } => {
                let mut annots = IndexMap::new();
                for a in annotations {
                    let a_val = self.eval_node(a, env)?;
                    annots.insert(a.to_string_key(), a_val);
                }
                let func = AvelynFunc {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                    closure: env.clone(),
                    variadic: *variadic,
                    annotations: annots,
                };
                let val = AvelynVal::Func(Rc::new(func));
                env.declare(name, val.clone());
                Ok(val)
            }

            ASTNode::Lambda { params, body, variadic, annotations } => {
                let mut annots = IndexMap::new();
                for a in annotations {
                    let a_val = self.eval_node(a, env)?;
                    annots.insert(a.to_string_key(), a_val);
                }
                let func = AvelynFunc {
                    name: None,
                    params: params.clone(),
                    body: body.clone(),
                    closure: env.clone(),
                    variadic: *variadic,
                    annotations: annots,
                };
                Ok(AvelynVal::Func(Rc::new(func)))
            }

            ASTNode::FuncCall { name, args } => {
                let callee = env.get(name).ok_or_else(|| Signal::Error(AvelynError::fmt(format!("NameError: function '{}' not defined", name))))?;
                self.eval_call_with_args(&callee, args, env)
            }

            ASTNode::CallExpr { callee, args } => {
                let callee_val = self.eval_node(callee, env)?;
                self.eval_call_with_args(&callee_val, args, env)
            }

            ASTNode::PrintCall(arg_node) => {
                let val = self.eval_node(arg_node, env)?;
                println!("{}", val.format());
                Ok(AvelynVal::Null)
            }

            ASTNode::TimeCall => builtins::native_time(self, vec![]).map_err(Signal::Error),

            ASTNode::While { cond, body } => {
                // Auto-parallelize while loops: while var < limit (for limits >= 100,000)
                if let ASTNode::BinOp { left, op, right } = cond.as_ref() {
                    if op == "<" || op == "<=" {
                        if let ASTNode::Var(ref var_name) = left.as_ref() {
                            let limit_val = self.eval_node(right, env)?;
                            let target_limit = limit_val.as_i64();
                            let current_start = env.get(var_name).map(|v| v.as_i64()).unwrap_or(0);
                            
                            if target_limit - current_start >= 100000 {
                                let total_iterations = target_limit - current_start;
                                let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
                                let chunk = total_iterations / (num_threads as i64);

                                let current_file = self.current_file.clone();
                                let body_clone = body.clone();
                                let var_name_clone = var_name.clone();

                                let mut handles = Vec::new();
                                for t in 0..num_threads {
                                    let f = current_file.clone();
                                    let b = body_clone.clone();
                                    let v = var_name_clone.clone();
                                    let start_range = current_start + (t as i64) * chunk;
                                    let end_range = if t == num_threads - 1 { target_limit } else { current_start + ((t + 1) as i64) * chunk };

                                    let handle = std::thread::Builder::new()
                                        .stack_size(128 * 1024 * 1024)
                                        .spawn(move || {
                                            let mut thread_interp = Interpreter::new();
                                            thread_interp.current_file = f;
                                            let thread_env = Env::new();

                                            let mut cur = start_range;
                                            while cur < end_range {
                                                thread_env.declare(&v, AvelynVal::Int(cur));
                                                for stmt in &b {
                                                    match thread_interp.eval_node(stmt, &thread_env) {
                                                        Ok(_) => {}
                                                        Err(Signal::Break) => break,
                                                        Err(Signal::Continue) => break,
                                                        Err(_) => {}
                                                    }
                                                }
                                                cur += 1;
                                            }
                                        }).ok();
                                    if let Some(h) = handle { handles.push(h); }
                                }

                                for h in handles { h.join().ok(); }
                                env.set(var_name, AvelynVal::Int(target_limit));
                                return Ok(AvelynVal::Null);
                            }
                        }
                    }
                }

                while self.eval_node(cond, env)?.is_truthy() {
                    for stmt in body {
                        match self.eval_node(stmt, env) {
                            Ok(_) => {}
                            Err(Signal::Break) => return Ok(AvelynVal::Null),
                            Err(Signal::Continue) => break,
                            Err(other) => return Err(other),
                        }
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::For { var, iter, body } => {
                let iter_val = self.eval_node(iter, env)?;
                let items: Vec<AvelynVal> = match iter_val {
                    AvelynVal::List(l) => l.borrow().clone(),
                    AvelynVal::ByteArray(b) => b.borrow().iter().map(|byte| AvelynVal::Int(*byte as i64)).collect(),
                    AvelynVal::Str(s) => s.chars().map(|c| AvelynVal::str(c.to_string())).collect(),
                    AvelynVal::Map(m) => m.borrow().keys().map(|k| AvelynVal::str(k.clone())).collect(),
                    _ => vec![],
                };

                for item in items {
                    env.declare(var, item);
                    for stmt in body {
                        match self.eval_node(stmt, env) {
                            Ok(_) => {}
                            Err(Signal::Break) => return Ok(AvelynVal::Null),
                            Err(Signal::Continue) => break,
                            Err(other) => return Err(other),
                        }
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::ForRange { var, from, to, inclusive, body } => {
                let start = self.eval_node(from, env)?.as_i64();
                let end = self.eval_node(to, env)?.as_i64();
                let total = (end - start).abs();

                if total >= 100000 {
                    let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
                    let chunk = total / (num_threads as i64);

                    let current_file = self.current_file.clone();
                    let body_clone = body.clone();
                    let var_clone = var.clone();

                    let mut handles = Vec::new();
                    for t in 0..num_threads {
                        let f = current_file.clone();
                        let b = body_clone.clone();
                        let v = var_clone.clone();
                        let start_range = start + (t as i64) * chunk;
                        let end_range = if t == num_threads - 1 { end } else { start + ((t + 1) as i64) * chunk };

                        let handle = std::thread::Builder::new()
                            .stack_size(128 * 1024 * 1024)
                            .spawn(move || {
                                let mut thread_interp = Interpreter::new();
                                thread_interp.current_file = f;
                                let thread_env = Env::new();

                                let mut cur = start_range;
                                while if start <= end { cur < end_range } else { cur > end_range } {
                                    thread_env.declare(&v, AvelynVal::Int(cur));
                                    for stmt in &b {
                                        match thread_interp.eval_node(stmt, &thread_env) {
                                            Ok(_) => {}
                                            Err(Signal::Break) => break,
                                            Err(Signal::Continue) => break,
                                            Err(_) => {}
                                        }
                                    }
                                    if start <= end { cur += 1; } else { cur -= 1; }
                                }
                            }).ok();
                        if let Some(h) = handle { handles.push(h); }
                    }

                    for h in handles { h.join().ok(); }
                    return Ok(AvelynVal::Null);
                }

                let mut i = start;
                let step = if start <= end { 1 } else { -1 };
                loop {
                    if (step > 0 && (if *inclusive { i > end } else { i >= end })) ||
                       (step < 0 && (if *inclusive { i < end } else { i <= end })) {
                        break;
                    }
                    env.declare(var, AvelynVal::Int(i));
                    for stmt in body {
                        match self.eval_node(stmt, env) {
                            Ok(_) => {}
                            Err(Signal::Break) => return Ok(AvelynVal::Null),
                            Err(Signal::Continue) => break,
                            Err(other) => return Err(other),
                        }
                    }
                    i += step;
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::If { cond, then, els } => {
                let c = self.eval_node(cond, env)?;
                if c.is_truthy() {
                    for stmt in then { self.eval_node(stmt, env)?; }
                } else if let Some(else_stmts) = els {
                    for stmt in else_stmts { self.eval_node(stmt, env)?; }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::Switch { subject, cases } => {
                let s_val = self.eval_node(subject, env)?;
                for (pat_opt, body) in cases {
                    let matches = match pat_opt {
                        Some(ASTNode::BinOp { left, op, right }) if op == ".." || op == "..." => {
                            let s_num = self.eval_node(left, env)?.as_f64();
                            let e_num = self.eval_node(right, env)?.as_f64();
                            let v_num = s_val.as_f64();
                            if op == "..." { v_num >= s_num && v_num <= e_num }
                            else { v_num >= s_num && v_num < e_num }
                        }
                        Some(pat) => self.eval_node(pat, env)?.deep_equal(&s_val),
                        None => true, // default case
                    };
                    if matches {
                        for stmt in body { self.eval_node(stmt, env)?; }
                        break;
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::Match { subject, arms } => {
                let s_val = self.eval_node(subject, env)?;
                for (pattern, body) in arms {
                    let match_env = Env::child(env.clone());
                    if self.pattern_match(pattern, &s_val, &match_env) {
                        for stmt in body { self.eval_node(stmt, &match_env)?; }
                        return Ok(AvelynVal::Null);
                    }
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::Return(expr) => {
                let val = self.eval_node(expr, env)?;
                Err(Signal::Return(val))
            }

            ASTNode::Break => Err(Signal::Break),
            ASTNode::Continue => Err(Signal::Continue),
            ASTNode::Pass => Ok(AvelynVal::Null),

            ASTNode::Throw(expr) => {
                let val = self.eval_node(expr, env)?;
                Err(Signal::Error(AvelynError::new(val)))
            }

            ASTNode::TryCatch { body, catches, finally_body } => {
                let res = (|| {
                    for stmt in body { self.eval_node(stmt, env)?; }
                    Ok(AvelynVal::Null)
                })();

                let mut res = res;
                if let Err(Signal::Error(err)) = &res {
                    let mut handled = false;
                    for (type_filter, catch_var, catch_body) in catches {
                        let matches = match type_filter {
                            None => true,
                            Some(name) => {
                                match &err.val {
                                    AvelynVal::Instance(inst) => inst.borrow().type_name == *name,
                                    AvelynVal::Variant(v) => v.type_name == *name,
                                    AvelynVal::Str(s) => s.as_ref() == name,
                                    _ => false,
                                }
                            }
                        };

                        if matches {
                            let catch_env = Env::child(env.clone());
                            catch_env.declare(catch_var, err.val.clone());
                            for stmt in catch_body { self.eval_node(stmt, &catch_env)?; }
                            res = Ok(AvelynVal::Null);
                            handled = true;
                            break;
                        }
                    }
                    if !handled { return res; }
                }

                if let Some(fin_stmts) = finally_body {
                    for stmt in fin_stmts { self.eval_node(stmt, env)?; }
                }
                res
            }

            ASTNode::Assert { cond, msg } => {
                let c = self.eval_node(cond, env)?;
                if !c.is_truthy() {
                    let m = if let Some(m_node) = msg { self.eval_node(m_node, env)?.format() } else { "Assertion failed".into() };
                    return Err(Signal::Error(AvelynError::fmt(format!("AssertionError: {}", m))));
                }
                Ok(AvelynVal::Null)
            }

            ASTNode::Import(path) => {
                let source_info = match self.module_manager.resolve(&path, &self.current_file) {
                    Ok(s) => s,
                    Err(e) => return Err(Signal::Error(AvelynError::fmt(e))),
                };

                let (content, resolved_path, cache_key) = match source_info {
                    ModuleSource::File(p) => {
                        let key = p.to_string_lossy().to_string();
                        if let Some(m) = self.module_cache.get(&key) { return Ok(m.clone()); }
                        let c = std::fs::read_to_string(&p).map_err(|e| Signal::Error(AvelynError::fmt(format!("IOError: {}", e))))?;
                        (c, p, key)
                    }
                    ModuleSource::Embedded(c) => {
                        let key = format!("embedded://{}", path);
                        if let Some(m) = self.module_cache.get(&key) { return Ok(m.clone()); }
                        (c, std::path::PathBuf::from(&key), key)
                    }
                };

                if let Err(e) = self.module_manager.enter_loading(resolved_path.clone()) {
                    return Err(Signal::Error(AvelynError::fmt(e)));
                }

                let module_env = Env::child(self.globals.clone());
                let module = Rc::new(RefCell::new(crate::value::Module {
                    name: path.clone(),
                    env: module_env.clone(),
                    exports: HashSet::new(),
                }));

                let old_file = self.current_file.clone();
                let old_module = self.current_module.clone();

                self.current_file = resolved_path.to_string_lossy().to_string();
                self.current_module = Some(module.clone());

                let mut lexer = crate::lexer::Lexer::new(&content);
                let tokens = lexer.tokenize();
                let mut parser = crate::parser::Parser::new(tokens);
                let ast = parser.parse();

                let res = self.eval_ast_with_env(&ast, &module_env);

                self.current_file = old_file;
                self.current_module = old_module;
                self.module_manager.exit_loading();

                res.map_err(Signal::Error)?;

                let mod_val = AvelynVal::Module(module);
                self.module_cache.insert(cache_key, mod_val.clone());
                self.module_manager.mark_loaded(resolved_path);

                // Auto-bind to module name if used as a statement
                let mod_name = std::path::Path::new(&path).file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone());
                env.declare(&mod_name, mod_val.clone());

                Ok(mod_val)
            }

            ASTNode::Include(path) => {
                let source_info = match self.module_manager.resolve(&path, &self.current_file) {
                    Ok(s) => s,
                    Err(e) => return Err(Signal::Error(AvelynError::fmt(e))),
                };

                let (content, resolved_path) = match source_info {
                    ModuleSource::File(p) => {
                        let c = std::fs::read_to_string(&p).map_err(|e| Signal::Error(AvelynError::fmt(format!("IOError: {}", e))))?;
                        (c, p)
                    }
                    ModuleSource::Embedded(c) => {
                        (c, std::path::PathBuf::from(format!("embedded://{}", path)))
                    }
                };

                if let Err(e) = self.module_manager.enter_loading(resolved_path.clone()) {
                    return Err(Signal::Error(AvelynError::fmt(e)));
                }

                let old_file = self.current_file.clone();
                self.current_file = resolved_path.to_string_lossy().to_string();

                let mut lexer = crate::lexer::Lexer::new(&content);
                let tokens = lexer.tokenize();
                let mut parser = crate::parser::Parser::new(tokens);
                let ast = parser.parse();

                let res = self.eval_ast_with_env(&ast, env);

                self.current_file = old_file;
                self.module_manager.exit_loading();
                self.module_manager.mark_loaded(resolved_path);

                res.map_err(Signal::Error)?;
                Ok(AvelynVal::Null)
            }

            ASTNode::Export(inner) => {
                let res = self.eval_node(inner, env)?;
                if let Some(curr) = &self.current_module {
                    match inner.as_ref() {
                        ASTNode::Decl { name, .. } | ASTNode::FuncDecl { name, .. } |
                        ASTNode::StructDecl { name, .. } | ASTNode::EnumDecl { name, .. } => {
                            curr.borrow_mut().exports.insert(name.clone());
                        }
                        _ => {}
                    }
                }
                Ok(res)
            }
        }
    }

    fn eval_call_with_args(&mut self, callee: &AvelynVal, args: &[ASTNode], env: &Rc<Env>) -> Result<AvelynVal, Signal> {
        let mut positional = Vec::new();
        let mut named = HashMap::new();

        for arg_node in args {
            if let ASTNode::NamedArg { name: n, value: v } = arg_node {
                let val = self.eval_node(v, env)?;
                named.insert(n.clone(), val);
            } else if let ASTNode::Spread(inner) = arg_node {
                let val = self.eval_node(inner, env)?;
                if let AvelynVal::List(l) = val {
                    positional.extend(l.borrow().iter().cloned());
                }
            } else {
                positional.push(self.eval_node(arg_node, env)?);
            }
        }

        match callee {
            AvelynVal::Native(n_fn) => n_fn(self, positional).map_err(Signal::Error),
            AvelynVal::Variant(v) if v.values.is_empty() => {
                // If it's an empty variant being called, it's a constructor call
                Ok(AvelynVal::Variant(Rc::new(crate::value::EnumVariantInstance {
                    type_name: v.type_name.clone(),
                    variant_name: v.variant_name.clone(),
                    values: positional,
                })))
            }
            AvelynVal::Type(def) => match def {
                crate::value::TypeDefinition::Struct { name, fields, .. } => {
                    let mut inst_fields = IndexMap::new();
                    for (i, f_name) in fields.iter().enumerate() {
                        let val = positional.get(i).cloned().unwrap_or(AvelynVal::Null);
                        inst_fields.insert(f_name.clone(), val);
                    }
                    // Also handle named args
                    for (k, v) in named { inst_fields.insert(k, v); }
                    Ok(AvelynVal::Instance(Rc::new(std::cell::RefCell::new(crate::value::StructInstance {
                        type_name: name.clone(),
                        fields: inst_fields,
                    }))))
                }
                crate::value::TypeDefinition::Enum { name, variants: _, .. } => {
                    Err(Signal::Error(AvelynError::fmt(format!("TypeError: enum '{}' cannot be called directly", name))))
                }
            }
            AvelynVal::Func(f) => {
                let call_env = Env::child(f.closure.clone());
                if f.variadic && !f.params.is_empty() {
                    let fixed_count = f.params.len() - 1;
                    for i in 0..fixed_count {
                        let param_name = &f.params[i].0;
                        let val = if let Some(n_val) = named.get(param_name) {
                            n_val.clone()
                        } else if i < positional.len() {
                            positional[i].clone()
                        } else if let Some(def_node) = &f.params[i].1 {
                            self.eval_node(def_node, &f.closure)?
                        } else {
                            AvelynVal::Null
                        };
                        call_env.declare(param_name, val);
                    }
                    let var_param_name = &f.params[fixed_count].0;
                    let rest = if positional.len() > fixed_count {
                        positional[fixed_count..].to_vec()
                    } else {
                        vec![]
                    };
                    call_env.declare(var_param_name, AvelynVal::list(rest));
                } else {
                    for (i, (param_name, default_opt)) in f.params.iter().enumerate() {
                        let val = if let Some(n_val) = named.get(param_name) {
                            n_val.clone()
                        } else if i < positional.len() {
                            positional[i].clone()
                        } else if let Some(def_node) = default_opt {
                            self.eval_node(def_node, &f.closure)?
                        } else {
                            AvelynVal::Null
                        };
                        call_env.declare(param_name, val);
                    }
                }

                for stmt in &f.body {
                    match self.eval_node(stmt, &call_env) {
                        Ok(_) => {}
                        Err(Signal::Return(v)) => return Ok(v),
                        Err(other) => return Err(other),
                    }
                }
                Ok(AvelynVal::Null)
            }
            _ => Err(Signal::Error(AvelynError::fmt(format!("TypeError: '{}' is not callable", callee.type_name())))),
        }
    }

    pub fn call_func(&mut self, callee: &AvelynVal, args: Vec<AvelynVal>) -> Result<AvelynVal, Signal> {
        match callee {
            AvelynVal::Native(n_fn) => n_fn(self, args).map_err(Signal::Error),
            AvelynVal::Func(f) => {
                let call_env = Env::child(f.closure.clone());
                for (i, (name, default_opt)) in f.params.iter().enumerate() {
                    let val = if i < args.len() {
                        args[i].clone()
                    } else if let Some(def_node) = default_opt {
                        self.eval_node(def_node, &f.closure)?
                    } else {
                        AvelynVal::Null
                    };
                    call_env.declare(name, val);
                }

                for stmt in &f.body {
                    match self.eval_node(stmt, &call_env) {
                        Ok(_) => {}
                        Err(Signal::Return(v)) => return Ok(v),
                        Err(other) => return Err(other),
                    }
                }
                Ok(AvelynVal::Null)
            }
            _ => Err(Signal::Error(AvelynError::fmt(format!("TypeError: '{}' is not callable", callee.type_name())))),
        }
    }

    fn eval_bin_op(&self, l: &AvelynVal, op: &str, r: &AvelynVal) -> Result<AvelynVal, Signal> {
        match op {
            "+" => match (l, r) {
                (AvelynVal::Int(a), AvelynVal::Int(b)) => Ok(AvelynVal::Int(a + b)),
                (AvelynVal::Float(a), AvelynVal::Float(b)) => Ok(AvelynVal::Float(a + b)),
                (AvelynVal::Int(a), AvelynVal::Float(b)) => Ok(AvelynVal::Float(*a as f64 + b)),
                (AvelynVal::Float(a), AvelynVal::Int(b)) => Ok(AvelynVal::Float(a + *b as f64)),
                (AvelynVal::Str(a), AvelynVal::Str(b)) => Ok(AvelynVal::str(format!("{}{}", a, b))),
                (AvelynVal::Str(a), b) => Ok(AvelynVal::str(format!("{}{}", a, b.format()))),
                (a, AvelynVal::Str(b)) => Ok(AvelynVal::str(format!("{}{}", a.format(), b))),
                (AvelynVal::List(a), AvelynVal::List(b)) => {
                    let mut res = a.borrow().clone();
                    res.extend(b.borrow().iter().cloned());
                    Ok(AvelynVal::list(res))
                }
                _ => Ok(AvelynVal::Null),
            },
            "-" => match (l, r) {
                (AvelynVal::Int(a), AvelynVal::Int(b)) => Ok(AvelynVal::Int(a.wrapping_sub(*b))),
                (AvelynVal::Float(a), AvelynVal::Float(b)) => Ok(AvelynVal::Float(a - b)),
                (AvelynVal::Int(a), AvelynVal::Float(b)) => Ok(AvelynVal::Float(*a as f64 - b)),
                (AvelynVal::Float(a), AvelynVal::Int(b)) => Ok(AvelynVal::Float(a - *b as f64)),
                _ => Ok(AvelynVal::Float(l.as_f64() - r.as_f64())),
            },
            "*" => Ok(match (l, r) {
                (AvelynVal::Str(s), AvelynVal::Int(n)) => AvelynVal::str(s.repeat((*n as usize).max(0))),
                (AvelynVal::Int(n), AvelynVal::Str(s)) => AvelynVal::str(s.repeat((*n as usize).max(0))),
                (AvelynVal::Int(a), AvelynVal::Int(b)) => AvelynVal::Int(a.wrapping_mul(*b)),
                _ => AvelynVal::Float(l.as_f64() * r.as_f64()),
            }),
            "/" => {
                let denom = r.as_f64();
                Ok(AvelynVal::Float(l.as_f64() / denom))
            }
            "//" => match (l, r) {
                (AvelynVal::Int(a), AvelynVal::Int(b)) => {
                    if *b == 0 { return Err(Signal::Error(AvelynError::msg("ZeroDivisionError: integer division by zero"))); }
                    Ok(AvelynVal::Int(a.div_euclid(*b)))
                }
                _ => Ok(AvelynVal::Float((l.as_f64() / r.as_f64()).floor())),
            },
            "%" => match (l, r) {
                (AvelynVal::Int(a), AvelynVal::Int(b)) => {
                    if *b == 0 { return Err(Signal::Error(AvelynError::msg("ZeroDivisionError: modulo by zero"))); }
                    Ok(AvelynVal::Int(a.rem_euclid(*b)))
                }
                _ => {
                    let bf = r.as_f64();
                    Ok(AvelynVal::Float(l.as_f64() % bf))
                }
            },
            "**" => Ok(AvelynVal::Float(l.as_f64().powf(r.as_f64()))),

            "==" => Ok(AvelynVal::Bool(l.deep_equal(r))),
            "!=" => Ok(AvelynVal::Bool(!l.deep_equal(r))),
            "<"  => Ok(AvelynVal::Bool(l.as_f64() < r.as_f64())),
            ">"  => Ok(AvelynVal::Bool(l.as_f64() > r.as_f64())),
            "<=" => Ok(AvelynVal::Bool(l.as_f64() <= r.as_f64())),
            ">=" => Ok(AvelynVal::Bool(l.as_f64() >= r.as_f64())),

            "&"  => Ok(AvelynVal::Int(l.as_i64() & r.as_i64())),
            "|"  => Ok(AvelynVal::Int(l.as_i64() | r.as_i64())),
            "^"  => Ok(AvelynVal::Int(l.as_i64() ^ r.as_i64())),
            "<<" => Ok(AvelynVal::Int(l.as_i64().wrapping_shl((r.as_i64() & 63) as u32))),
            ">>" => Ok(AvelynVal::Int(l.as_i64().wrapping_shr((r.as_i64() & 63) as u32))),
            ">>>" => Ok(AvelynVal::Int(((l.as_i64() as u64).wrapping_shr((r.as_i64() & 63) as u32)) as i64)),

            _ => Ok(AvelynVal::Null),
        }
    }

    fn pattern_match(&self, pattern: &crate::ast::Pattern, value: &AvelynVal, env: &Rc<Env>) -> bool {
        match pattern {
            crate::ast::Pattern::Wildcard => true,
            crate::ast::Pattern::Literal(node) => {
                if let crate::ast::ASTNode::BinOp { left, op, right } = node {
                    if op == ".." || op == "..." {
                        let s_num = match left.as_ref() { crate::ast::ASTNode::Int(i) => *i as f64, crate::ast::ASTNode::Float(f) => *f, _ => 0.0 };
                        let e_num = match right.as_ref() { crate::ast::ASTNode::Int(i) => *i as f64, crate::ast::ASTNode::Float(f) => *f, _ => 0.0 };
                        let v_num = value.as_f64();
                        if op == "..." { return v_num >= s_num && v_num <= e_num; }
                        else { return v_num >= s_num && v_num < e_num; }
                    }
                }
                match (node, value) {
                    (crate::ast::ASTNode::Int(i), AvelynVal::Int(v)) => *i == *v,
                    (crate::ast::ASTNode::Float(f), AvelynVal::Float(v)) => *f == *v,
                    (crate::ast::ASTNode::Str(s), AvelynVal::Str(v)) => s == v.as_ref(),
                    (crate::ast::ASTNode::Bool(b), AvelynVal::Bool(v)) => *b == *v,
                    (crate::ast::ASTNode::Null, AvelynVal::Null) => true,
                    _ => false,
                }
            }
            crate::ast::Pattern::Var(name) => {
                env.declare(name, value.clone());
                true
            }
            crate::ast::Pattern::List(pats) => {
                if let AvelynVal::List(l) = value {
                    let vec = l.borrow();
                    if vec.len() != pats.len() { return false; }
                    for (p, v) in pats.iter().zip(vec.iter()) {
                        if !self.pattern_match(p, v, env) { return false; }
                    }
                    true
                } else { false }
            }
            crate::ast::Pattern::Struct { name, fields } => {
                if let AvelynVal::Instance(inst) = value {
                    let inst = inst.borrow();
                    if &inst.type_name != name { return false; }
                    for (f_name, p) in fields {
                        if let Some(v) = inst.fields.get(f_name) {
                            if !self.pattern_match(p, v, env) { return false; }
                        } else { return false; }
                    }
                    true
                } else { false }
            }
            crate::ast::Pattern::Enum { type_name, variant, args } => {
                if let AvelynVal::Variant(v_inst) = value {
                    if &v_inst.type_name == type_name && &v_inst.variant_name == variant {
                        if v_inst.values.len() != args.len() { return false; }
                        for (p, v) in args.iter().zip(v_inst.values.iter()) {
                            if !self.pattern_match(p, v, env) { return false; }
                        }
                        return true;
                    }
                }
                false
            }
            crate::ast::Pattern::Or(pats) => {
                for p in pats {
                    if self.pattern_match(p, value, env) { return true; }
                }
                false
            }
        }
    }
}
