#![allow(dead_code, unused_imports, unused_variables)]
// airgen/mod.rs — AST → AIR lowering
//
// AirGen lowers a (type-annotated) AST into an AirModule.  It mirrors the
// logic that was previously in `llvm_codegen.rs`, but instead of emitting
// raw LLVM IR strings it constructs structured AIR instructions through the
// AirBuilder API.
//
// The resulting AIR can be:
//   • Verified by `air::verify`
//   • Optimized by `optimizer`
//   • Printed by `air::printer` (--emit-air / --emit-air-opt)
//   • Lowered to LLVM IR text by `irgen`

pub mod builder;
pub mod runtime_map;

use std::collections::{HashMap, HashSet};
use crate::ast::{ASTNode, Span};
use crate::air::{
    AirFunction, AirModule, AirParam, AirType, BasicBlock, BlockId,
    Inst, RuntimeFn, Value, VOID_VALUE, BinOpCode, UnaryOpCode,
};
use crate::sema::diagnostics::DiagnosticEmitter;
use builder::AirBuilder;

pub struct AirGen<'a> {
    diag:           &'a mut DiagnosticEmitter,
    module:         AirModule,
    user_fn_names:    HashSet<String>,
    fn_param_names:   HashMap<String, Vec<String>>,
    variadic_fns:     HashSet<String>,
    lowered_fn_names: HashSet<String>,
    global_vars:      HashSet<String>,
    immutable_vars:   HashSet<String>,
    lambda_count:     usize,
}

impl<'a> AirGen<'a> {
    pub fn new(diag: &'a mut DiagnosticEmitter) -> Self {
        AirGen {
            diag,
            module: AirModule::new("avelyn_module"),
            user_fn_names: HashSet::new(),
            fn_param_names: HashMap::new(),
            variadic_fns: HashSet::new(),
            lowered_fn_names: HashSet::new(),
            global_vars: HashSet::new(),
            immutable_vars: HashSet::new(),
            lambda_count: 0,
        }
    }

    fn is_known_runtime_builtin(name: &str) -> bool {
        matches!(name,
            "print" | "println" | "toNumber" | "toBool" | "isNumber" | "isNull" | "isString" | "isArray" | "isMap" | "isInteger" | "isBool" |
            "arrayLen" | "stringLen" | "len" | "arrayAppend" | "arrayPush" | "arrayPop" | "arrayIndexOf" | "arrayContains" | "arrayRemove" | "arraySlice" |
            "stringSplit" | "stringConcat" | "stringSub" | "stringReverse" | "stringEndsWith" | "stringStartsWith" | "stringContains" | "stringUpper" | "stringLower" | "stringTrim" | "stringReplace" |
            "mathSqrt" | "mathRound" | "mathPow" | "mathAbs" | "mathFloor" | "mathCeil" | "mapGet" | "mapSet" | "mapHas" | "mapKeys" | "mapValues" |
            "fileWrite" | "fileRead" | "sysRemoveFile" | "numCpus" | "timeSec" | "timeMs" | "timeSleep" | "random" | "randint" | "choice" | "tokenHex" |
            "sysSecureRandomDouble" | "sysSecureRandomBytes" | "getAtIndex" | "jsonStringify" | "square" | "Queue" | "Stack" | "double" | "cube" | "assert" | "spawnWorkers" | "dateNow" | "numToString" | "sysEnv" |
            "pathJoin" | "pathBasename" | "pathDirname" | "pathExtension" | "pathAbsolute" | "fileAppend" | "fileExists" | "dirCreate" | "dirExists" | "dirList" | "dirRemove" | "rmTree" | "copyTree" |
            "stringAt" | "stringIndexOf" | "stringJoin" | "stringPadStart" | "stringRepeat" | "stringReplaceAll" | "stringSplitLines" | "stringToLower" | "stringToUpper" | "strip" |
            "randomBytes" | "randomInt" | "dateFormat" | "dateParse" | "dateAdd" | "sysArch" | "sysPlatform" | "sysArgv" | "sysCopyFile" | "sysMoveFile" | "sysExecute" | "sysExit" | "sysReadLine" | "sysRegexFindAll" | "sysRegexGroups" | "sysRegexMatch" | "sysRegexReplace" | "sysUrlParse" | "uuidV4" |
            "aesEncrypt" | "aesDecrypt" | "hmac" | "sha512" | "entropy" | "httpBasicBrute" | "httpDirBrute" | "httpRequest" | "netAccept" | "netClose" | "netConnect" | "netDnsLookup" | "netGrabBanner" | "netListen" | "netPortScan" | "netRead" | "netRecv" | "netRecvFrom" | "netSend" | "netSendTo" | "netSetNonBlocking" | "netSetTimeout" | "netUdpBind" | "netUdpSocket" | "netWrite" | "webCreate" | "webRoute" | "webServe" |
            "arrayCopy" | "arrayReverse" | "arrayShift" | "arraySort" | "mapCopy" | "deepEqual" | "mathSin" | "mathCos" | "mathTan" | "mathLog" | "mathLog2" | "mathLog10" | "mathExp" | "mathMin" | "mathMax" | "base64Encode" | "base64Decode" | "hexEncode" | "hexDecode" | "jsonParse" | "urlEncode" | "urlDecode"
        )
    }

    /// Helper: strip ASTNode::Line wrappers to get the underlying semantic node
    fn unwrap_line(node: &ASTNode) -> &ASTNode {
        match node {
            ASTNode::Line(_, inner) => Self::unwrap_line(inner),
            _ => node,
        }
    }

    /// Lower a full program AST to an AirModule.
    pub fn lower(mut self, ast: &[ASTNode]) -> Result<AirModule, Vec<String>> {
        // Pre-collect user function names for CallExpr dispatch.
        self.collect_user_fns(ast);
        self.collect_global_vars(ast);

        self.lambda_count = 0;

        // Lower all function declarations (including nested ones).
        self.lower_all_func_decls(ast)?;

        // Lower the top-level statements into a synthetic `@main` function.
        self.lower_main(ast)?;

        self.module.rebuild_all_cfgs();
        Ok(self.module)
    }

    fn collect_global_vars(&mut self, nodes: &[ASTNode]) {
        for node in nodes {
            let unwrap = Self::unwrap_line(node);
            match unwrap {
                ASTNode::Decl { name, mutable, .. } => {
                    if !self.global_vars.contains(name) {
                        self.global_vars.insert(name.clone());
                        if !mutable { self.immutable_vars.insert(name.clone()); }
                        self.module.globals.push(crate::air::AirGlobal {
                            name: name.clone(),
                            ty: AirType::SylvelVal,
                            init: None,
                        });
                    }
                }
                ASTNode::DestructureArray { names, mutable, .. } => {
                    for name_opt in names {
                        if let Some(name) = name_opt {
                            if !self.global_vars.contains(name) {
                                self.global_vars.insert(name.clone());
                                if !mutable { self.immutable_vars.insert(name.clone()); }
                                self.module.globals.push(crate::air::AirGlobal {
                                    name: name.clone(),
                                    ty: AirType::SylvelVal,
                                    init: None,
                                });
                            }
                        }
                    }
                }
                ASTNode::DestructureMap { keys, mutable, .. } => {
                    for (k, alias) in keys {
                        let name = alias.as_ref().unwrap_or(k);
                        if !self.global_vars.contains(name) {
                            self.global_vars.insert(name.clone());
                            if !mutable { self.immutable_vars.insert(name.clone()); }
                            self.module.globals.push(crate::air::AirGlobal {
                                name: name.clone(),
                                ty: AirType::SylvelVal,
                                init: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn get_global_ptr(&self, name: &str, b: &mut AirBuilder) -> Value {
        let sym = format!("@lyn_var_{}", name);
        if let Some((v, _)) = b.func.global_val_map.iter().find(|(_, s)| *s == &sym) {
            *v
        } else {
            let v = b.fresh_value();
            b.func.global_val_map.insert(v, sym);
            v
        }
    }

    // ── Pre-pass: collect function names ──────────────────────────────────

    fn collect_user_fns(&mut self, nodes: &[ASTNode]) {
        for node in nodes {
            self.collect_fns_rec(node);
        }
        // Sync into the module.
        self.module.user_fn_names = self.user_fn_names.clone();
    }

    fn collect_fns_rec(&mut self, node: &ASTNode) {
        match node {
            ASTNode::FuncDecl { name, params, body, variadic, .. } => {
                self.user_fn_names.insert(name.clone());
                let pnames: Vec<String> = params.iter().map(|p| p.0.clone()).collect();
                self.fn_param_names.insert(name.clone(), pnames.clone());
                self.fn_param_names.insert(format!("lyn_fn_{}", name), pnames);
                if *variadic {
                    self.variadic_fns.insert(name.clone());
                    self.variadic_fns.insert(format!("lyn_fn_{}", name));
                }
                for s in body { self.collect_fns_rec(s); }
            }
            ASTNode::Lambda { params, body, .. } => {
                let lname = format!("__lambda_{}", self.lambda_count);
                self.lambda_count += 1;
                self.user_fn_names.insert(lname.clone());
                let pnames: Vec<String> = params.iter().map(|p| p.0.clone()).collect();
                self.fn_param_names.insert(lname.clone(), pnames.clone());
                self.fn_param_names.insert(format!("lyn_fn_{}", lname), pnames);
                for s in body { self.collect_fns_rec(s); }
            }
            ASTNode::Decl { value, .. } | ASTNode::Assign { value, .. } => {
                self.collect_fns_rec(value);
            }
            ASTNode::If { cond, then, els } => {
                self.collect_fns_rec(cond);
                for s in then { self.collect_fns_rec(s); }
                if let Some(e) = els { for s in e { self.collect_fns_rec(s); } }
            }
            ASTNode::While { cond, body } => {
                self.collect_fns_rec(cond);
                for s in body { self.collect_fns_rec(s); }
            }
            ASTNode::For { iter, body, .. } | ASTNode::ForRange { from: iter, body, .. } => {
                self.collect_fns_rec(iter);
                for s in body { self.collect_fns_rec(s); }
            }
            ASTNode::ArrayLit(items) => { for i in items { self.collect_fns_rec(i); } }
            ASTNode::MapLit(pairs)   => { for (k, v) in pairs { self.collect_fns_rec(k); self.collect_fns_rec(v); } }
            ASTNode::Line(_, inner)  => { self.collect_fns_rec(inner); }
            _ => {}
        }
    }

    // ── Main function synthesis ───────────────────────────────────────────

    fn lower_main(&mut self, ast: &[ASTNode]) -> Result<(), Vec<String>> {
        let mut builder = AirBuilder::new("main", Span::UNKNOWN);
        builder.func.ret_ty = AirType::I64; // int main() returns i32/i64

        // Pre-register top-level variable allocations and globals.
        for g_name in &self.global_vars {
            let g_ptr = self.get_global_ptr(g_name, &mut builder);
            builder.set_var(g_name, g_ptr);
        }

        // Lower each top-level statement (skip FuncDecl, already lowered).
        for node in ast {
            let unwrap = Self::unwrap_line(node);
            if !matches!(unwrap, ASTNode::FuncDecl { .. }) {
                self.lower_node(node, &mut builder)?;
            }
        }

        // Return 0.
        let zero = builder.fresh_value();
        builder.emit(Inst::ConstInt(zero, 0));
        builder.emit_return(zero);

        let func = builder.finalize();
        self.module.add_function(func);
        Ok(())
    }

    // ── Function declaration lowering ─────────────────────────────────────

    fn lower_all_func_decls(&mut self, nodes: &[ASTNode]) -> Result<(), Vec<String>> {
        for node in nodes {
            let unwrap = Self::unwrap_line(node);
            match unwrap {
                ASTNode::FuncDecl { body, .. } => {
                    self.lower_func_decl(unwrap)?;
                    self.lower_all_func_decls(body)?;
                }
                ASTNode::If { then, els, .. } => {
                    self.lower_all_func_decls(then)?;
                    if let Some(e) = els { self.lower_all_func_decls(e)?; }
                }
                ASTNode::While { body, .. } | ASTNode::For { body, .. } | ASTNode::ForRange { body, .. } => {
                    self.lower_all_func_decls(body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn lower_func_decl(&mut self, node: &ASTNode) -> Result<(), Vec<String>> {
        let node = Self::unwrap_line(node);
        if let ASTNode::FuncDecl { name, params, body, .. } = node {
            let mangled = format!("lyn_fn_{}", name);
            if !self.lowered_fn_names.insert(mangled.clone()) {
                return Ok(());
            }
            let mut builder = AirBuilder::new(&mangled, Span::UNKNOWN);
            builder.func.ret_ty = AirType::Void;

            // First param is the output SylvelVal* pointer.
            let out_val = builder.fresh_value();
            builder.func.params.push(AirParam {
                name: "out".to_string(),
                value: out_val,
                ty: AirType::SylvelValPtr,
            });

            // Remaining params.
            for (pname, default_opt) in params {
                let pval = builder.fresh_value();
                builder.func.params.push(AirParam {
                    name: pname.clone(),
                    value: pval,
                    ty: AirType::SylvelValPtr,
                });
                builder.set_var(pname, pval);

                // Default parameter handling: if isNull(param), set default.
                if let Some(default_expr) = default_opt {
                    let is_null_val = builder.fresh_value();
                    builder.entry_allocs.push(Inst::Alloc(is_null_val, AirType::SylvelVal));
                    builder.emit_runtime_call_void(RuntimeFn::Builtin("isNull".to_string()), vec![is_null_val, pval]);
                    let is_null_bool = builder.emit_runtime_call(RuntimeFn::ToBool, vec![is_null_val], AirType::Bool);

                    let set_def_bb = builder.new_block("def_set");
                    let skip_def_bb = builder.new_block("def_skip");

                    builder.emit_branch(is_null_bool, set_def_bb, skip_def_bb);

                    builder.switch_to(set_def_bb);
                    let def_val = self.lower_node(default_expr, &mut builder)?;
                    builder.emit(Inst::Store(def_val, pval));
                    builder.emit_jump(skip_def_bb);

                    builder.switch_to(skip_def_bb);
                }
            }

            // Lower body.
            let mut body_returned = false;
            for stmt in body {
                let _ = self.lower_node(stmt, &mut builder)?;
                if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                    body_returned = true;
                    break;
                }
            }

            if !body_returned {
                // Implicit null return.
                builder.emit_runtime_call_void(RuntimeFn::MakeNull, vec![out_val]);
                builder.emit_return_void();
            }

            let func = builder.finalize();
            self.module.add_function(func);
            Ok(())
        } else {
            Err(vec!["Expected FuncDecl".to_string()])
        }
    }

    // ── Node lowering (main dispatcher) ───────────────────────────────────

    fn lower_node(&mut self, node: &ASTNode, b: &mut AirBuilder) -> Result<Value, Vec<String>> {
        // Unwrap Line wrapper and propagate source location.
        if let ASTNode::Line(line, inner) = node {
            b.emit_debug_loc(Span::from_line(*line));
            return self.lower_node(inner, b);
        }

        match node {
            // ── Literals ─────────────────────────────────────────────────
            ASTNode::Int(i)   => Ok(b.emit_const_int(*i)),
            ASTNode::Float(f) => Ok(b.emit_const_float(*f)),
            ASTNode::Bool(v)  => {
                let bv = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(bv, AirType::SylvelVal));
                let iv = b.fresh_value();
                b.emit(Inst::ConstBool(iv, *v));
                b.emit_runtime_call_void(RuntimeFn::MakeBool, vec![bv, iv]);
                Ok(bv)
            }
            ASTNode::FuncDecl { .. } => {
                self.lower_func_decl(node)?;
                let nv = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                Ok(nv)
            }
            ASTNode::Null => {
                let nv = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                Ok(nv)
            }
            ASTNode::Str(s) => Ok(b.emit_const_str(s, &mut self.module)),
            ASTNode::ByteArray(bytes) => {
                let list = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(list, AirType::SylvelVal));
                let cap = b.fresh_value();
                b.emit(Inst::ConstInt(cap, bytes.len() as i64));
                b.emit_runtime_call_void(RuntimeFn::AllocList, vec![list, cap]);
                for byte in bytes {
                    let bv = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(bv, AirType::SylvelVal));
                    let iv = b.fresh_value();
                    b.emit(Inst::ConstInt(iv, *byte as i64));
                    b.emit_runtime_call_void(RuntimeFn::MakeInt, vec![bv, iv]);
                    b.emit_runtime_call_void(RuntimeFn::ListPush, vec![list, bv]);
                }
                Ok(list)
            }

            // ── Variable references ───────────────────────────────────────
            ASTNode::Var(name) => {
                if let Some(v) = b.lookup_var(name) {
                    Ok(v)
                } else if self.global_vars.contains(name) {
                    let g_ptr = self.get_global_ptr(name, b);
                    Ok(g_ptr)
                } else {
                    // Unknown variable — produce null and warn.
                    self.diag.warning(Span::UNKNOWN,
                        format!("use of undefined variable '{}' in codegen", name));
                    let nv = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                    Ok(nv)
                }
            }

            // ── Variable declarations ─────────────────────────────────────
            ASTNode::Decl { name, value, mutable, .. } => {
                let val = self.lower_node(value, b)?;
                if !mutable { b.mark_immutable(name); }
                let slot = if b.func.name == "main" && self.global_vars.contains(name) {
                    let g_ptr = self.get_global_ptr(name, b);
                    b.set_var(name, g_ptr);
                    g_ptr
                } else if let Some(existing) = b.lookup_var(name) {
                    existing
                } else {
                    let s = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(s, AirType::SylvelVal));
                    b.set_var(name, s);
                    s
                };
                // Copy val → slot via a store in AIR.
                b.emit(Inst::Store(val, slot));
                Ok(slot)
            }

            // ── Assignment ────────────────────────────────────────────────
            ASTNode::Assign { name, value } => {
                if b.is_immutable(name) || self.immutable_vars.contains(name) {
                    let msg = format!("ImmutabilityError: cannot assign to immutable binding '{}' declared with 'let'", name);
                    let msg_idx = self.module.intern_string(&msg);
                    let msg_val = b.fresh_value();
                    b.emit(Inst::ConstStr(msg_val, msg_idx));
                    b.emit_runtime_call_void(RuntimeFn::RaiseError, vec![msg_val]);
                    let null = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                    return Ok(null);
                }
                let val = self.lower_node(value, b)?;
                let slot = if let Some(s) = b.lookup_var(name) {
                    s
                } else if self.global_vars.contains(name) {
                    self.get_global_ptr(name, b)
                } else {
                    let s = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(s, AirType::SylvelVal));
                    b.set_var(name, s);
                    s
                };
                b.emit(Inst::Store(val, slot));
                Ok(slot)
            }

            // ── Compound assignment ───────────────────────────────────────
            ASTNode::CompoundAssign { name, op, value } => {
                let bin_node = ASTNode::BinOp {
                    left: Box::new(ASTNode::Var(name.clone())),
                    op: op.clone(),
                    right: value.clone(),
                };
                let assign_node = ASTNode::Assign {
                    name: name.clone(),
                    value: Box::new(bin_node),
                };
                self.lower_node(&assign_node, b)
            }

            // ── Array Destructuring ───────────────────────────────────────
            ASTNode::DestructureArray { names, value, mutable } => {
                let arr_val = self.lower_node(value, b)?;
                for (i, name_opt) in names.iter().enumerate() {
                    if let Some(name) = name_opt {
                        let elem_val = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(elem_val, AirType::SylvelVal));
                        let idx_val = b.fresh_value();
                        b.emit(Inst::ConstInt(idx_val, i as i64));
                        b.emit_runtime_call_void(RuntimeFn::ListGet, vec![elem_val, arr_val, idx_val]);

                        let var_slot = if b.func.name == "main" && self.global_vars.contains(name) {
                            let g_ptr = self.get_global_ptr(name, b);
                            b.set_var(name, g_ptr);
                            g_ptr
                        } else if let Some(vs) = b.lookup_var(name) {
                            vs
                        } else {
                            let vs = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(vs, AirType::SylvelVal));
                            b.set_var(name, vs);
                            vs
                        };
                        b.emit(Inst::Store(elem_val, var_slot));
                        if !mutable {
                            self.immutable_vars.insert(name.clone());
                            b.mark_immutable(name);
                        }
                    }
                }
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Map Destructuring ─────────────────────────────────────────
            ASTNode::DestructureMap { keys, value, mutable } => {
                let map_val = self.lower_node(value, b)?;
                for (k, alias_opt) in keys {
                    let var_name = alias_opt.as_deref().unwrap_or(k.as_str());
                    let k_val = b.emit_const_str(k, &mut self.module);
                    let elem_val = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(elem_val, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MapGet, vec![elem_val, map_val, k_val]);

                    let var_slot = if b.func.name == "main" && self.global_vars.contains(var_name) {
                        let g_ptr = self.get_global_ptr(var_name, b);
                        b.set_var(var_name, g_ptr);
                        g_ptr
                    } else if let Some(vs) = b.lookup_var(var_name) {
                        vs
                    } else {
                        let vs = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(vs, AirType::SylvelVal));
                        b.set_var(var_name, vs);
                        vs
                    };
                    b.emit(Inst::Store(elem_val, var_slot));
                    if !mutable {
                        self.immutable_vars.insert(var_name.to_string());
                        b.mark_immutable(var_name);
                    }
                }
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Index assignment ──────────────────────────────────────────
            ASTNode::IndexAssign { target, index, value } => {
                if b.is_immutable(target) {
                    let msg = format!("ImmutabilityError: cannot assign to immutable binding '{}'", target);
                    let msg_idx = self.module.intern_string(&msg);
                    let msg_val = b.fresh_value();
                    b.emit(Inst::ConstStr(msg_val, msg_idx));
                    b.emit_runtime_call_void(RuntimeFn::RaiseError, vec![msg_val]);
                    let null = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                    return Ok(null);
                }
                let target_val = if let Some(v) = b.lookup_var(target) {
                    v
                } else if self.global_vars.contains(target) {
                    self.get_global_ptr(target, b)
                } else {
                    let v = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(v, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![v]);
                    v
                };
                let idx_val = self.lower_node(index, b)?;
                let new_val = self.lower_node(value, b)?;
                b.emit_runtime_call_void(RuntimeFn::SubscriptSet, vec![target_val, idx_val, new_val]);
                Ok(new_val)
            }

            // ── Print ─────────────────────────────────────────────────────
            ASTNode::PrintCall(expr) => {
                let val = self.lower_node(expr, b)?;
                b.emit_runtime_call_void(RuntimeFn::Print, vec![val]);
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Assert ───────────────────────────────────────────────────
            ASTNode::Assert { cond, msg } => {
                let cond_val = self.lower_node(cond, b)?;
                let msg_val  = if let Some(m) = msg {
                    self.lower_node(m, b)?
                } else {
                    let nv = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                    nv
                };
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::BuiltinAssert, vec![res, cond_val, msg_val]);
                Ok(res)
            }

            // ── Binary operations ─────────────────────────────────────────
            ASTNode::BinOp { left, op, right } => {
                let l = self.lower_node(left, b)?;
                let r = self.lower_node(right, b)?;
                let op_code = runtime_map::binop_code(op);
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                let op_val = b.fresh_value();
                b.emit(Inst::ConstInt(op_val, op_code as i64));
                b.emit_runtime_call_void(RuntimeFn::BinOp, vec![res, l, op_val, r]);
                Ok(res)
            }

            // ── Unary operations ──────────────────────────────────────────
            ASTNode::UnaryOp { op, operand } => {
                let val = self.lower_node(operand, b)?;
                let op_code = runtime_map::unaryop_code(op);
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                let op_val = b.fresh_value();
                b.emit(Inst::ConstInt(op_val, op_code as i64));
                b.emit_runtime_call_void(RuntimeFn::UnaryOp, vec![res, op_val, val]);
                Ok(res)
            }

            // ── Ternary expression ────────────────────────────────────────
            ASTNode::Ternary { cond, then, els } => {
                let cond_val = self.lower_node(cond, b)?;
                let bool_val = b.emit_runtime_call(RuntimeFn::ToBool, vec![cond_val], AirType::Bool);

                let then_bb  = b.new_block("tern_then");
                let else_bb  = b.new_block("tern_else");
                let merge_bb = b.new_block("tern_merge");

                b.emit_branch(bool_val, then_bb, else_bb);

                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));

                b.switch_to(then_bb);
                let t_val = self.lower_node(then, b)?;
                b.emit(Inst::Store(t_val, res));
                b.emit_jump(merge_bb);

                b.switch_to(else_bb);
                let e_val = self.lower_node(els, b)?;
                b.emit(Inst::Store(e_val, res));
                b.emit_jump(merge_bb);

                b.switch_to(merge_bb);
                Ok(res)
            }

            // ── Null coalesce ─────────────────────────────────────────────
            ASTNode::NullCoalesce { left, right } => {
                let l_val = self.lower_node(left, b)?;
                // Check strictly if left is null via isNull.
                let is_null_val = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(is_null_val, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::Builtin("isNull".to_string()), vec![is_null_val, l_val]);
                let is_null_bool = b.emit_runtime_call(RuntimeFn::ToBool, vec![is_null_val], AirType::Bool);

                let null_bb    = b.new_block("nc_null");
                let nonnull_bb = b.new_block("nc_nonnull");
                let merge_bb   = b.new_block("nc_merge");

                b.emit_branch(is_null_bool, null_bb, nonnull_bb);

                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));

                b.switch_to(null_bb);
                let r_val = self.lower_node(right, b)?;
                b.emit(Inst::Store(r_val, res));
                b.emit_jump(merge_bb);

                b.switch_to(nonnull_bb);
                b.emit(Inst::Store(l_val, res));
                b.emit_jump(merge_bb);

                b.switch_to(merge_bb);
                Ok(res)
            }

            // ── If statement ──────────────────────────────────────────────
            ASTNode::If { cond, then, els } => {
                let cond_val = self.lower_node(cond, b)?;
                let bool_val = b.emit_runtime_call(RuntimeFn::ToBool, vec![cond_val], AirType::Bool);

                let then_bb  = b.new_block("then");
                let else_bb  = b.new_block("else");
                let merge_bb = b.new_block("merge");

                let has_else = els.is_some();
                if has_else {
                    b.emit_branch(bool_val, then_bb, else_bb);
                } else {
                    b.emit_branch(bool_val, then_bb, merge_bb);
                }

                b.switch_to(then_bb);
                b.push_scope();
                let mut then_terminated = false;
                for stmt in then {
                    let _ = self.lower_node(stmt, b)?;
                    if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                        then_terminated = true;
                        break;
                    }
                }
                b.pop_scope();
                if !then_terminated { b.emit_jump(merge_bb); }

                if let Some(else_stmts) = els {
                    b.switch_to(else_bb);
                    b.push_scope();
                    let mut else_terminated = false;
                    for stmt in else_stmts {
                        let _ = self.lower_node(stmt, b)?;
                        if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                            else_terminated = true;
                            break;
                        }
                    }
                    b.pop_scope();
                    if !else_terminated { b.emit_jump(merge_bb); }
                }

                b.switch_to(merge_bb);
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── While loop ────────────────────────────────────────────────
            ASTNode::While { cond, body } => {
                let cond_bb = b.new_block("while_cond");
                let body_bb = b.new_block("while_body");
                let exit_bb = b.new_block("while_exit");

                b.emit_jump(cond_bb);
                b.push_loop(exit_bb, cond_bb);

                b.switch_to(cond_bb);
                let cond_val = self.lower_node(cond, b)?;
                let bool_val = b.emit_runtime_call(RuntimeFn::ToBool, vec![cond_val], AirType::Bool);
                b.emit_branch(bool_val, body_bb, exit_bb);

                b.switch_to(body_bb);
                b.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.lower_node(stmt, b)?;
                    if matches!(stmt, ASTNode::Break | ASTNode::Continue | ASTNode::Return(_)) {
                        body_terminated = true;
                        break;
                    }
                }
                b.pop_scope();
                if !body_terminated { b.emit_jump(cond_bb); }

                b.pop_loop();
                b.switch_to(exit_bb);

                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── For range ─────────────────────────────────────────────────
            ASTNode::ForRange { var, from, to, inclusive, body } => {
                let from_val = self.lower_node(from, b)?;
                let to_val   = self.lower_node(to, b)?;

                let start_i = b.emit_runtime_call(RuntimeFn::ToInt, vec![from_val], AirType::I64);
                let end_i   = b.emit_runtime_call(RuntimeFn::ToInt, vec![to_val],   AirType::I64);

                // Index counter (i64 alloc).
                let idx_slot = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(idx_slot, AirType::I64));
                b.emit(Inst::Store(start_i, idx_slot));

                // SylvelVal slot for the loop variable.
                let var_slot = if let Some(vs) = b.lookup_var(var) {
                    vs
                } else {
                    let vs = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(vs, AirType::SylvelVal));
                    b.set_var(var, vs);
                    vs
                };

                let cond_bb = b.new_block("for_cond");
                let body_bb = b.new_block("for_body");
                let step_bb = b.new_block("for_step");
                let exit_bb = b.new_block("for_exit");

                b.push_loop(exit_bb, step_bb);
                b.emit_jump(cond_bb);

                b.switch_to(cond_bb);
                let curr_i = b.fresh_value();
                b.emit(Inst::Load(curr_i, idx_slot));
                let cmp = if *inclusive {
                    b.fresh_value()
                } else {
                    b.fresh_value()
                };
                b.emit(if *inclusive {
                    Inst::ICmpSle(cmp, curr_i, end_i)
                } else {
                    Inst::ICmpSlt(cmp, curr_i, end_i)
                });
                b.emit_branch(cmp, body_bb, exit_bb);

                b.switch_to(body_bb);
                // Set loop variable to current index.
                let curr_i2 = b.fresh_value();
                b.emit(Inst::Load(curr_i2, idx_slot));
                b.emit_runtime_call_void(RuntimeFn::MakeInt, vec![var_slot, curr_i2]);

                b.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.lower_node(stmt, b)?;
                    if matches!(stmt, ASTNode::Break | ASTNode::Continue | ASTNode::Return(_)) {
                        body_terminated = true;
                        break;
                    }
                }
                b.pop_scope();
                if !body_terminated { b.emit_jump(step_bb); }

                b.switch_to(step_bb);
                let curr_i3 = b.fresh_value();
                b.emit(Inst::Load(curr_i3, idx_slot));
                let one = b.fresh_value();
                b.emit(Inst::ConstInt(one, 1));
                let next_i = b.fresh_value();
                b.emit(Inst::IAdd(next_i, curr_i3, one));
                b.emit(Inst::Store(next_i, idx_slot));
                b.emit_jump(cond_bb);

                b.pop_loop();
                b.switch_to(exit_bb);

                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── For each ─────────────────────────────────────────────────
            ASTNode::For { var, iter, body } => {
                let iter_val = self.lower_node(iter, b)?;
                let len_val  = b.emit_runtime_call(RuntimeFn::Len, vec![iter_val], AirType::I64);

                let idx_slot = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(idx_slot, AirType::I64));
                let zero = b.fresh_value();
                b.emit(Inst::ConstInt(zero, 0));
                b.emit(Inst::Store(zero, idx_slot));

                let var_slot = if let Some(vs) = b.lookup_var(var) {
                    vs
                } else {
                    let vs = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(vs, AirType::SylvelVal));
                    b.set_var(var, vs);
                    vs
                };

                let cond_bb = b.new_block("for_iter_cond");
                let body_bb = b.new_block("for_iter_body");
                let step_bb = b.new_block("for_iter_step");
                let exit_bb = b.new_block("for_iter_exit");

                b.push_loop(exit_bb, step_bb);
                b.emit_jump(cond_bb);

                b.switch_to(cond_bb);
                let curr_i = b.fresh_value();
                b.emit(Inst::Load(curr_i, idx_slot));
                let cmp = b.fresh_value();
                b.emit(Inst::ICmpSlt(cmp, curr_i, len_val));
                b.emit_branch(cmp, body_bb, exit_bb);

                b.switch_to(body_bb);
                // subscript_get(var_slot, iter_val, idx_val)
                let idx_sv = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(idx_sv, AirType::SylvelVal));
                let curr_i2 = b.fresh_value();
                b.emit(Inst::Load(curr_i2, idx_slot));
                b.emit_runtime_call_void(RuntimeFn::MakeInt, vec![idx_sv, curr_i2]);
                b.emit_runtime_call_void(RuntimeFn::SubscriptGet, vec![var_slot, iter_val, idx_sv]);

                b.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.lower_node(stmt, b)?;
                    if matches!(stmt, ASTNode::Break | ASTNode::Continue | ASTNode::Return(_)) {
                        body_terminated = true;
                        break;
                    }
                }
                b.pop_scope();
                if !body_terminated { b.emit_jump(step_bb); }

                b.switch_to(step_bb);
                let curr_i3 = b.fresh_value();
                b.emit(Inst::Load(curr_i3, idx_slot));
                let one2 = b.fresh_value();
                b.emit(Inst::ConstInt(one2, 1));
                let next_i = b.fresh_value();
                b.emit(Inst::IAdd(next_i, curr_i3, one2));
                b.emit(Inst::Store(next_i, idx_slot));
                b.emit_jump(cond_bb);

                b.pop_loop();
                b.switch_to(exit_bb);

                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Return ────────────────────────────────────────────────────
            ASTNode::Return(expr) => {
                let val = self.lower_node(expr, b)?;
                // Store into the function's out param (first param = %v0).
                if let Some(out_param) = b.func.params.first() {
                    let out_v = out_param.value;
                    b.emit(Inst::Store(val, out_v));
                }
                b.emit_return_void();
                Ok(val)
            }

            // ── Break / Continue ──────────────────────────────────────────
            ASTNode::Break => {
                if let Some(exit_bb) = b.loop_break_target() {
                    b.emit_jump(exit_bb);
                }
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                Ok(null)
            }
            ASTNode::Continue => {
                if let Some(cont_bb) = b.loop_continue_target() {
                    b.emit_jump(cont_bb);
                }
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                Ok(null)
            }

            // ── Array literal ─────────────────────────────────────────────
            ASTNode::ArrayLit(items) => {
                let list = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(list, AirType::SylvelVal));
                let cap = b.fresh_value();
                b.emit(Inst::ConstInt(cap, items.len() as i64));
                b.emit_runtime_call_void(RuntimeFn::AllocList, vec![list, cap]);
                for item in items {
                    if let ASTNode::Spread(inner) = item {
                        // Spread: iterate and push each element.
                        let inner_val = self.lower_node(inner, b)?;
                        let spread_node = ASTNode::For {
                            var: "__spread_item__".to_string(),
                            iter: Box::new(ASTNode::Var("__spread_src__".to_string())),
                            body: vec![],
                        };
                        // Simplified: call len + loop to push.
                        let inner_len = b.emit_runtime_call(RuntimeFn::Len, vec![inner_val], AirType::I64);
                        let sidx = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(sidx, AirType::I64));
                        let s0 = b.fresh_value();
                        b.emit(Inst::ConstInt(s0, 0));
                        b.emit(Inst::Store(s0, sidx));

                        let sc_bb = b.new_block("spread_cond");
                        let sb_bb = b.new_block("spread_body");
                        let se_bb = b.new_block("spread_exit");
                        b.emit_jump(sc_bb);
                        b.switch_to(sc_bb);
                        let sci = b.fresh_value();
                        b.emit(Inst::Load(sci, sidx));
                        let scmp = b.fresh_value();
                        b.emit(Inst::ICmpSlt(scmp, sci, inner_len));
                        b.emit_branch(scmp, sb_bb, se_bb);
                        b.switch_to(sb_bb);
                        let sitem = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(sitem, AirType::SylvelVal));
                        let sidx_sv = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(sidx_sv, AirType::SylvelVal));
                        let sci2 = b.fresh_value();
                        b.emit(Inst::Load(sci2, sidx));
                        b.emit_runtime_call_void(RuntimeFn::MakeInt, vec![sidx_sv, sci2]);
                        b.emit_runtime_call_void(RuntimeFn::ListGet, vec![sitem, inner_val, sci2]);
                        b.emit_runtime_call_void(RuntimeFn::ListPush, vec![list, sitem]);
                        let sone = b.fresh_value();
                        b.emit(Inst::ConstInt(sone, 1));
                        let snext = b.fresh_value();
                        b.emit(Inst::IAdd(snext, sci2, sone));
                        b.emit(Inst::Store(snext, sidx));
                        b.emit_jump(sc_bb);
                        b.switch_to(se_bb);
                    } else {
                        let item_val = self.lower_node(item, b)?;
                        b.emit_runtime_call_void(RuntimeFn::ListPush, vec![list, item_val]);
                    }
                }
                Ok(list)
            }

            // ── Map literal ───────────────────────────────────────────────
            ASTNode::MapLit(pairs) => {
                let map = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(map, AirType::SylvelVal));
                let cap = b.fresh_value();
                b.emit(Inst::ConstInt(cap, pairs.len() as i64));
                b.emit_runtime_call_void(RuntimeFn::AllocMap, vec![map, cap]);
                for (k, v) in pairs {
                    let kv = self.lower_node(k, b)?;
                    let vv = self.lower_node(v, b)?;
                    b.emit_runtime_call_void(RuntimeFn::MapSet, vec![map, kv, vv]);
                }
                Ok(map)
            }

            // ── Subscript ─────────────────────────────────────────────────
            ASTNode::Subscript { target, index } => {
                let target_val = self.lower_node(target, b)?;
                let index_val  = self.lower_node(index, b)?;
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::SubscriptGet, vec![res, target_val, index_val]);
                Ok(res)
            }

            // ── Function call (named) ─────────────────────────────────────
            ASTNode::FuncCall { name, args } => {
                let is_var = b.lookup_var(name).is_some() || self.global_vars.contains(name);
                if is_var {
                    let var_node = ASTNode::Var(name.clone());
                    return self.lower_node(&ASTNode::CallExpr { callee: Box::new(var_node), args: args.clone() }, b);
                }

                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));

                let is_variadic = self.variadic_fns.contains(name) || self.variadic_fns.contains(&format!("lyn_fn_{}", name));
                if is_variadic {
                    let mangled = if name.starts_with("lyn_fn_") { name.clone() } else { format!("lyn_fn_{}", name) };
                    let pcount = self.fn_param_names.get(name).map(|p| p.len()).unwrap_or(1);
                    let mut air_args = vec![res];
                    let regular_count = if pcount > 0 { pcount - 1 } else { 0 };
                    for i in 0..regular_count {
                        if i < args.len() {
                            air_args.push(self.lower_node(&args[i], b)?);
                        } else {
                            let nv = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                            b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                            air_args.push(nv);
                        }
                    }
                    let var_list = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(var_list, AirType::SylvelVal));
                    let excess_count = if args.len() > regular_count { args.len() - regular_count } else { 0 };
                    let cap = b.fresh_value();
                    b.emit(Inst::ConstInt(cap, excess_count as i64));
                    b.emit_runtime_call_void(RuntimeFn::AllocList, vec![var_list, cap]);
                    for i in regular_count..args.len() {
                        let arg_unwrapped = Self::unwrap_line(&args[i]);
                        if let ASTNode::Spread(inner) = arg_unwrapped {
                            let spread_list = self.lower_node(inner, b)?;
                            let len_val = b.emit_runtime_call(RuntimeFn::Len, vec![spread_list], AirType::I64);
                            let idx_ptr = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(idx_ptr, AirType::I64));
                            let zero = b.fresh_value();
                            b.emit(Inst::ConstInt(zero, 0));
                            b.emit(Inst::Store(zero, idx_ptr));

                            let cond_bb = b.new_block("spread_cond");
                            let body_bb = b.new_block("spread_body");
                            let exit_bb = b.new_block("spread_exit");
                            b.emit_jump(cond_bb);

                            b.switch_to(cond_bb);
                            let cur_idx = b.fresh_value();
                            b.emit(Inst::Load(cur_idx, idx_ptr));
                            let has_more = b.fresh_value();
                            b.emit(Inst::ICmpSlt(has_more, cur_idx, len_val));
                            b.emit_branch(has_more, body_bb, exit_bb);

                            b.switch_to(body_bb);
                            let elem = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(elem, AirType::SylvelVal));
                            b.emit_runtime_call_void(RuntimeFn::ListGet, vec![elem, spread_list, cur_idx]);
                            b.emit_runtime_call_void(RuntimeFn::ListPush, vec![var_list, elem]);
                            let one = b.fresh_value();
                            b.emit(Inst::ConstInt(one, 1));
                            let next_idx = b.fresh_value();
                            b.emit(Inst::IAdd(next_idx, cur_idx, one));
                            b.emit(Inst::Store(next_idx, idx_ptr));
                            b.emit_jump(cond_bb);

                            b.switch_to(exit_bb);
                        } else {
                            let elem = self.lower_node(&args[i], b)?;
                            b.emit_runtime_call_void(RuntimeFn::ListPush, vec![var_list, elem]);
                        }
                    }
                    air_args.push(var_list);
                    b.emit(Inst::Call(VOID_VALUE, mangled, air_args));
                    return Ok(res);
                }

                if let Some(param_names) = self.fn_param_names.get(name).cloned() {
                    let mangled = if name.starts_with("lyn_fn_") { name.clone() } else { format!("lyn_fn_{}", name) };
                    let mut ordered_args: Vec<Option<Value>> = vec![None; param_names.len()];
                    let mut pos_idx = 0;
                    for arg in args {
                        let unwrap_arg = Self::unwrap_line(arg);
                        if let ASTNode::NamedArg { name: narg_name, value: narg_val } = unwrap_arg {
                            if let Some(target_idx) = param_names.iter().position(|p| p == narg_name) {
                                let v = self.lower_node(narg_val, b)?;
                                ordered_args[target_idx] = Some(v);
                            }
                        } else {
                            let v = self.lower_node(arg, b)?;
                            if pos_idx < ordered_args.len() {
                                ordered_args[pos_idx] = Some(v);
                                pos_idx += 1;
                            }
                        }
                    }
                    for slot in &mut ordered_args {
                        if slot.is_none() {
                            let nv = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                            b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                            *slot = Some(nv);
                        }
                    }
                    let mut air_args = vec![res];
                    for v in ordered_args.into_iter().flatten() {
                        air_args.push(v);
                    }
                    b.emit(Inst::Call(VOID_VALUE, mangled, air_args));
                    return Ok(res);
                } else if self.user_fn_names.contains(name) || self.user_fn_names.contains(&format!("lyn_fn_{}", name)) {
                    let mut air_args = vec![res];
                    for arg in args {
                        let av = self.lower_node(arg, b)?;
                        air_args.push(av);
                    }
                    let mangled = if name.starts_with("lyn_fn_") { name.clone() } else { format!("lyn_fn_{}", name) };
                    b.emit(Inst::Call(VOID_VALUE, mangled, air_args));
                    return Ok(res);
                } else if Self::is_known_runtime_builtin(name) {
                    let mut air_args = vec![res];
                    for arg in args {
                        let av = self.lower_node(arg, b)?;
                        air_args.push(av);
                    }
                    let rt_fn = RuntimeFn::Builtin(name.clone());
                    let arity  = args.len();
                    self.module.extern_fns
                        .entry(rt_fn.c_name())
                        .or_insert(arity);
                    b.emit_runtime_call_void(rt_fn, air_args);
                    return Ok(res);
                } else {
                    let var_node = ASTNode::Var(name.clone());
                    return self.lower_node(&ASTNode::CallExpr { callee: Box::new(var_node), args: args.clone() }, b);
                }
            }

            // ── Call expression (first-class callable) ────────────────────
            ASTNode::CallExpr { callee, args } => {
                let callee_unwrap = Self::unwrap_line(callee);
                if let ASTNode::Var(name) = callee_unwrap {
                    let is_var = b.lookup_var(name).is_some() || self.global_vars.contains(name);
                    if !is_var {
                        let is_variadic = self.variadic_fns.contains(name) || self.variadic_fns.contains(&format!("lyn_fn_{}", name));
                        if is_variadic {
                            let mangled = if name.starts_with("lyn_fn_") { name.clone() } else { format!("lyn_fn_{}", name) };
                            let pcount = self.fn_param_names.get(name).map(|p| p.len()).unwrap_or(1);
                            let res = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                            let mut air_args = vec![res];
                            let regular_count = if pcount > 0 { pcount - 1 } else { 0 };
                            for i in 0..regular_count {
                                if i < args.len() {
                                    air_args.push(self.lower_node(&args[i], b)?);
                                } else {
                                    let nv = b.fresh_value();
                                    b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                                    air_args.push(nv);
                                }
                            }
                            let var_list = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(var_list, AirType::SylvelVal));
                            let excess_count = if args.len() > regular_count { args.len() - regular_count } else { 0 };
                            let cap = b.fresh_value();
                            b.emit(Inst::ConstInt(cap, excess_count as i64));
                            b.emit_runtime_call_void(RuntimeFn::AllocList, vec![var_list, cap]);
                            for i in regular_count..args.len() {
                                let arg_unwrapped = Self::unwrap_line(&args[i]);
                                if let ASTNode::Spread(inner) = arg_unwrapped {
                                    let spread_list = self.lower_node(inner, b)?;
                                    let len_val = b.emit_runtime_call(RuntimeFn::Len, vec![spread_list], AirType::I64);
                                    let idx_ptr = b.fresh_value();
                                    b.entry_allocs.push(Inst::Alloc(idx_ptr, AirType::I64));
                                    let zero = b.fresh_value();
                                    b.emit(Inst::ConstInt(zero, 0));
                                    b.emit(Inst::Store(zero, idx_ptr));

                                    let cond_bb = b.new_block("spread_cond");
                                    let body_bb = b.new_block("spread_body");
                                    let exit_bb = b.new_block("spread_exit");
                                    b.emit_jump(cond_bb);

                                    b.switch_to(cond_bb);
                                    let cur_idx = b.fresh_value();
                                    b.emit(Inst::Load(cur_idx, idx_ptr));
                                    let has_more = b.fresh_value();
                                    b.emit(Inst::ICmpSlt(has_more, cur_idx, len_val));
                                    b.emit_branch(has_more, body_bb, exit_bb);

                                    b.switch_to(body_bb);
                                    let elem = b.fresh_value();
                                    b.entry_allocs.push(Inst::Alloc(elem, AirType::SylvelVal));
                                    b.emit_runtime_call_void(RuntimeFn::ListGet, vec![elem, spread_list, cur_idx]);
                                    b.emit_runtime_call_void(RuntimeFn::ListPush, vec![var_list, elem]);
                                    let one = b.fresh_value();
                                    b.emit(Inst::ConstInt(one, 1));
                                    let next_idx = b.fresh_value();
                                    b.emit(Inst::IAdd(next_idx, cur_idx, one));
                                    b.emit(Inst::Store(next_idx, idx_ptr));
                                    b.emit_jump(cond_bb);

                                    b.switch_to(exit_bb);
                                } else {
                                    let elem = self.lower_node(&args[i], b)?;
                                    b.emit_runtime_call_void(RuntimeFn::ListPush, vec![var_list, elem]);
                                }
                            }
                            air_args.push(var_list);
                            b.emit(Inst::Call(VOID_VALUE, mangled, air_args));
                            return Ok(res);
                        }

                        if let Some(param_names) = self.fn_param_names.get(name).cloned() {
                            let res = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                            let mangled = if name.starts_with("lyn_fn_") { name.clone() } else { format!("lyn_fn_{}", name) };
                            let mut ordered_args: Vec<Option<Value>> = vec![None; param_names.len()];
                            let mut pos_idx = 0;
                            for arg in args {
                                let unwrap_arg = Self::unwrap_line(arg);
                                if let ASTNode::NamedArg { name: narg_name, value: narg_val } = unwrap_arg {
                                    if let Some(target_idx) = param_names.iter().position(|p| p == narg_name) {
                                        let v = self.lower_node(narg_val, b)?;
                                        ordered_args[target_idx] = Some(v);
                                    }
                                } else {
                                    let v = self.lower_node(arg, b)?;
                                    if pos_idx < ordered_args.len() {
                                        ordered_args[pos_idx] = Some(v);
                                        pos_idx += 1;
                                    }
                                }
                            }
                            for slot in &mut ordered_args {
                                if slot.is_none() {
                                    let nv = b.fresh_value();
                                    b.entry_allocs.push(Inst::Alloc(nv, AirType::SylvelVal));
                                    b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![nv]);
                                    *slot = Some(nv);
                                }
                            }
                            let mut air_args = vec![res];
                            for v in ordered_args.into_iter().flatten() {
                                air_args.push(v);
                            }
                            b.emit(Inst::Call(VOID_VALUE, mangled, air_args));
                            return Ok(res);
                        }
                    }
                }

                let callee_val = self.lower_node(callee, b)?;
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));

                let mut actual_args = vec![res];
                for a in args {
                    let av = self.lower_node(a, b)?;
                    actual_args.push(av);
                }

                let null_val = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null_val, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null_val]);

                // Dispatch: check each known user function by name (deduplicated).
                let mut seen_raw = HashSet::new();
                let mut user_fns = Vec::new();
                for fn_name in &self.user_fn_names {
                    let raw_name = fn_name.trim_start_matches("lyn_fn_");
                    if seen_raw.insert(raw_name.to_string()) {
                        user_fns.push(raw_name.to_string());
                    }
                }

                let end_bb = b.new_block("call_expr_end");
                let mut next_bb = b.new_block("call_user_check");
                b.emit_jump(next_bb);

                for raw_name in &user_fns {
                    let cur_bb  = next_bb;
                    next_bb     = b.new_block("call_user_check");
                    let hit_bb  = b.new_block("call_user_hit");

                    b.switch_to(cur_bb);
                    let fn_str = b.emit_const_str(raw_name, &mut self.module);
                    let cmp = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(cmp, AirType::SylvelVal));
                    let eq_code = b.fresh_value();
                    b.emit(Inst::ConstInt(eq_code, 6)); // BinOpCode::Eq
                    b.emit_runtime_call_void(RuntimeFn::BinOp, vec![cmp, callee_val, eq_code, fn_str]);
                    let is_hit = b.emit_runtime_call(RuntimeFn::ToBool, vec![cmp], AirType::Bool);
                    b.emit_branch(is_hit, hit_bb, next_bb);

                    b.switch_to(hit_bb);
                    let mangled = format!("lyn_fn_{}", raw_name);
                    let mut call_args = vec![res];
                    if let Some(pnames) = self.fn_param_names.get(raw_name) {
                        for i in 0..pnames.len() {
                            if i + 1 < actual_args.len() {
                                call_args.push(actual_args[i + 1]);
                            } else {
                                call_args.push(null_val);
                            }
                        }
                    } else {
                        for a in &actual_args[1..] {
                            call_args.push(*a);
                        }
                    }
                    b.emit(Inst::Call(VOID_VALUE, mangled, call_args));
                    b.emit_jump(end_bb);
                }

                b.switch_to(next_bb);
                let a1 = if actual_args.len() > 1 { actual_args[1] } else { null_val };
                let a2 = if actual_args.len() > 2 { actual_args[2] } else { null_val };
                b.emit_runtime_call_void(RuntimeFn::CallExpr, vec![res, callee_val, a1, a2]);
                b.emit_jump(end_bb);

                b.switch_to(end_bb);
                Ok(res)
            }

            // ── Lambda ────────────────────────────────────────────────────
            ASTNode::Lambda { params, body, .. } => {
                let lambda_name = format!("__lambda_{}", self.lambda_count);
                self.lambda_count += 1;
                let mangled = format!("lyn_fn_{}", lambda_name);
                let decl = ASTNode::FuncDecl {
                    name: lambda_name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    variadic: false,
                    annotations: vec![],
                };
                self.user_fn_names.insert(lambda_name.clone());
                self.user_fn_names.insert(mangled.clone());
                self.module.user_fn_names.insert(lambda_name.clone());
                self.module.user_fn_names.insert(mangled.clone());
                self.lower_func_decl(&decl)?;

                // Return a SylvelVal string containing the lambda name.
                let res = b.emit_const_str(&lambda_name, &mut self.module);
                Ok(res)
            }

            // ── Try / Catch ───────────────────────────────────────────────
            ASTNode::TryCatch { body, catches, finally_body } => {
                b.emit_runtime_call_void(RuntimeFn::EnterTry, vec![]);
                b.push_scope();

                let catch_bb  = b.new_block("try_catch");
                let normal_bb = b.new_block("try_normal");
                let end_bb    = b.new_block("try_end");

                let mut last = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(last, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![last]);

                for stmt in body {
                    last = self.lower_node(stmt, b)?;
                }

                let has_err_i = b.emit_runtime_call(RuntimeFn::HasError, vec![], AirType::I64);
                // Convert i32/i64 HasError result to bool: ne 0.
                let zero = b.fresh_value();
                b.emit(Inst::ConstInt(zero, 0));
                let is_err = b.fresh_value();
                b.emit(Inst::ICmpEq(is_err, has_err_i, zero)); // is_err = (has_err == 0) → no error
                // branch: if no error → normal_bb, else → catch_bb
                b.emit_branch(is_err, normal_bb, catch_bb);

                b.switch_to(normal_bb);
                b.emit_runtime_call_void(RuntimeFn::ExitTry, vec![]);
                b.emit_jump(end_bb);

                b.switch_to(catch_bb);
                b.emit_runtime_call_void(RuntimeFn::ExitTry, vec![]);
                b.emit_runtime_call_void(RuntimeFn::ClearError, vec![]);
                if let Some((_, var_name, catch_stmts)) = catches.first() {
                    let err_val = b.emit_const_str("caught", &mut self.module);
                    b.set_var(var_name, err_val);
                    for stmt in catch_stmts {
                        last = self.lower_node(stmt, b)?;
                    }
                }
                b.emit_jump(end_bb);

                b.switch_to(end_bb);
                b.pop_scope();

                if let Some(fin) = finally_body {
                    for stmt in fin {
                        let _ = self.lower_node(stmt, b)?;
                    }
                }
                Ok(last)
            }

            // ── Throw ─────────────────────────────────────────────────────
            ASTNode::Throw(expr) => {
                let val = self.lower_node(expr, b)?;
                b.emit_runtime_call_void(RuntimeFn::ThrowVal, vec![val]);
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Match ─────────────────────────────────────────────────────
            ASTNode::Match { subject, arms } => {
                let subj_val = self.lower_node(subject, b)?;
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![res]);

                let end_bb = b.new_block("match_end");
                let mut next_bb = b.new_block("match_check");
                b.emit_jump(next_bb);

                for (pattern, arm_body) in arms {
                    let cur_bb = next_bb;
                    next_bb    = b.new_block("match_check");
                    let hit_bb = b.new_block("match_hit");

                    b.switch_to(cur_bb);

                    match pattern {
                        crate::ast::Pattern::Wildcard => {
                            b.emit_jump(hit_bb);
                        }
                        crate::ast::Pattern::Literal(lit) => {
                            let lit_val = self.lower_node(lit, b)?;
                            let cmp = b.fresh_value();
                            b.entry_allocs.push(Inst::Alloc(cmp, AirType::SylvelVal));
                            let eq_code = b.fresh_value();
                            b.emit(Inst::ConstInt(eq_code, 6));
                            b.emit_runtime_call_void(RuntimeFn::BinOp, vec![cmp, subj_val, eq_code, lit_val]);
                            let is_hit = b.emit_runtime_call(RuntimeFn::ToBool, vec![cmp], AirType::Bool);
                            b.emit_branch(is_hit, hit_bb, next_bb);
                        }
                        _ => { b.emit_jump(hit_bb); }
                    }

                    b.switch_to(hit_bb);
                    b.push_scope();
                    let mut arm_last = res;
                    for stmt in arm_body {
                        arm_last = self.lower_node(stmt, b)?;
                    }
                    b.emit(Inst::Store(arm_last, res));
                    b.pop_scope();
                    b.emit_jump(end_bb);
                }

                b.switch_to(next_bb);
                b.emit_jump(end_bb);
                b.switch_to(end_bb);
                Ok(res)
            }

            // ── Spread (as expression) ────────────────────────────────────
            ASTNode::Spread(expr) => self.lower_node(expr, b),

            // ── Named argument ────────────────────────────────────────────
            ASTNode::NamedArg { value, .. } => self.lower_node(value, b),

            // ── Interpolated string ───────────────────────────────────────
            ASTNode::InterpStr(parts) => {
                if parts.is_empty() {
                    return Ok(b.emit_const_str("", &mut self.module));
                }
                let mut acc = self.lower_node(&parts[0], b)?;
                for part in &parts[1..] {
                    let pv = self.lower_node(part, b)?;
                    let res = b.fresh_value();
                    b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                    let add_code = b.fresh_value();
                    b.emit(Inst::ConstInt(add_code, 1)); // BinOpCode::Add
                    b.emit_runtime_call_void(RuntimeFn::BinOp, vec![res, acc, add_code, pv]);
                    acc = res;
                }
                Ok(acc)
            }

            // ── Pass / no-op ──────────────────────────────────────────────
            ASTNode::Pass | ASTNode::Import(_) | ASTNode::Include(_) | ASTNode::Export(_) => {
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }

            // ── Time call ─────────────────────────────────────────────────
            ASTNode::TimeCall => {
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::Builtin("timeSec".to_string()), vec![res]);
                Ok(res)
            }

            // ── Struct / Enum decl (no-op in native compile) ──────────────
            ASTNode::StructDecl { .. } | ASTNode::EnumDecl { .. } => {
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                Ok(null)
            }

            // ── Switch ────────────────────────────────────────────────────
            ASTNode::Switch { subject, cases } => {
                let subj_val = self.lower_node(subject, b)?;
                let res = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(res, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![res]);

                let end_bb = b.new_block("switch_end");
                let mut next_bb = b.new_block("switch_check");
                b.emit_jump(next_bb);

                for (case_opt, case_body) in cases {
                    let cur_bb = next_bb;
                    next_bb    = b.new_block("switch_check");
                    let hit_bb = b.new_block("switch_hit");

                    b.switch_to(cur_bb);
                    if let Some(case_expr) = case_opt {
                        let cv = self.lower_node(case_expr, b)?;
                        let cmp = b.fresh_value();
                        b.entry_allocs.push(Inst::Alloc(cmp, AirType::SylvelVal));
                        let eq_code = b.fresh_value();
                        b.emit(Inst::ConstInt(eq_code, 6));
                        b.emit_runtime_call_void(RuntimeFn::BinOp, vec![cmp, subj_val, eq_code, cv]);
                        let is_hit = b.emit_runtime_call(RuntimeFn::ToBool, vec![cmp], AirType::Bool);
                        b.emit_branch(is_hit, hit_bb, next_bb);
                    } else {
                        // Default case.
                        b.emit_jump(hit_bb);
                    }

                    b.switch_to(hit_bb);
                    b.push_scope();
                    let mut case_last = res;
                    for stmt in case_body {
                        case_last = self.lower_node(stmt, b)?;
                    }
                    b.emit(Inst::Store(case_last, res));
                    b.pop_scope();
                    b.emit_jump(end_bb);
                }

                b.switch_to(next_bb);
                b.emit_jump(end_bb);
                b.switch_to(end_bb);
                Ok(res)
            }

            _ => {
                // Fallback: produce null for unhandled nodes.
                let null = b.fresh_value();
                b.entry_allocs.push(Inst::Alloc(null, AirType::SylvelVal));
                b.emit_runtime_call_void(RuntimeFn::MakeNull, vec![null]);
                Ok(null)
            }
        }
    }
}

/// Public entry-point for the AIRGen stage.
pub fn lower_to_air(
    ast: &[ASTNode],
    diag: &mut DiagnosticEmitter,
) -> Result<AirModule, Vec<String>> {
    let gen = AirGen::new(diag);
    gen.lower(ast)
}
