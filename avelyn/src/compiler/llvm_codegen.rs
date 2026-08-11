// llvm_codegen.rs — LLVM IR Code Generator for Avelyn AST (Pointer ABI)

use std::collections::{HashMap, HashSet};
use crate::ast::ASTNode;

pub struct LLVMCodeGen {
    temp_count: usize,
    label_count: usize,
    lambda_count: usize,
    str_constants: Vec<(String, String)>, // (label, content)
    scopes: Vec<HashMap<String, String>>,  // var_name -> llvm_var_ptr
    current_loop_labels: Vec<(String, String)>, // (break_label, continue_label)
    current_func_out_ptr: Option<String>,
    user_func_names: HashSet<String>,
    declared_func_signatures: HashMap<String, usize>, // target_fn -> param_count
    entry_allocas: String,
    extra_user_funcs_ir: String,
}

impl LLVMCodeGen {
    pub fn new() -> Self {
        LLVMCodeGen {
            temp_count: 0,
            label_count: 0,
            lambda_count: 0,
            str_constants: Vec::new(),
            scopes: vec![HashMap::new()],
            current_loop_labels: Vec::new(),
            current_func_out_ptr: None,
            user_func_names: HashSet::new(),
            declared_func_signatures: HashMap::new(),
            entry_allocas: String::new(),
            extra_user_funcs_ir: String::new(),
        }
    }

    fn new_temp_ptr(&mut self, is_sylvel_val: bool) -> String {
        self.temp_count += 1;
        let ptr = format!("%t{}", self.temp_count);
        if is_sylvel_val {
            self.entry_allocas.push_str(&format!("  {} = alloca %SylvelVal\n", ptr));
        } else {
            self.entry_allocas.push_str(&format!("  {} = alloca i64\n", ptr));
        }
        ptr
    }

    fn new_label(&mut self, prefix: &str) -> String {
        self.label_count += 1;
        format!("{}_{}", prefix, self.label_count)
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn set_var(&mut self, name: &str, ptr: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ptr);
        }
    }

    fn lookup_var(&self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(ptr) = scope.get(name) {
                return Some(ptr.clone());
            }
        }
        None
    }

    fn add_string_constant(&mut self, str_val: &str) -> String {
        let label = format!("@str.{}", self.str_constants.len());
        self.str_constants.push((label.clone(), str_val.to_string()));
        label
    }

    fn collect_user_funcs_rec(&mut self, node: &ASTNode) {
        match node {
            ASTNode::FuncDecl { name, body, .. } => {
                self.user_func_names.insert(name.clone());
                for stmt in body { self.collect_user_funcs_rec(stmt); }
            }
            ASTNode::Lambda { body, .. } => {
                let lambda_name = format!("__lambda_{}", self.lambda_count);
                self.lambda_count += 1;
                self.user_func_names.insert(lambda_name);
                for stmt in body { self.collect_user_funcs_rec(stmt); }
            }
            ASTNode::Decl { value, .. } | ASTNode::Assign { name: _, value } => {
                self.collect_user_funcs_rec(value);
            }
            ASTNode::MapLit(pairs) => {
                for (k, v) in pairs {
                    self.collect_user_funcs_rec(k);
                    self.collect_user_funcs_rec(v);
                }
            }
            ASTNode::ArrayLit(items) => {
                for item in items { self.collect_user_funcs_rec(item); }
            }
            ASTNode::If { cond, then, els } => {
                self.collect_user_funcs_rec(cond);
                for stmt in then { self.collect_user_funcs_rec(stmt); }
                if let Some(else_stmts) = els {
                    for stmt in else_stmts { self.collect_user_funcs_rec(stmt); }
                }
            }
            ASTNode::While { cond, body } => {
                self.collect_user_funcs_rec(cond);
                for stmt in body { self.collect_user_funcs_rec(stmt); }
            }
            ASTNode::For { iter, body, .. } => {
                self.collect_user_funcs_rec(iter);
                for stmt in body { self.collect_user_funcs_rec(stmt); }
            }
            ASTNode::ForRange { from, to, body, .. } => {
                self.collect_user_funcs_rec(from);
                self.collect_user_funcs_rec(to);
                for stmt in body { self.collect_user_funcs_rec(stmt); }
            }
            _ => {}
        }
    }

    pub fn generate(&mut self, ast: &[ASTNode]) -> Result<String, String> {
        self.lambda_count = 0;
        for node in ast {
            self.collect_user_funcs_rec(node);
        }
        self.lambda_count = 0;

        self.entry_allocas.clear();

        // Pre-register top-level declared variables in global scope
        for node in ast {
            if let ASTNode::Decl { name, .. } = node {
                if self.lookup_var(name).is_none() {
                    let ptr = self.new_temp_ptr(true);
                    self.set_var(name, ptr);
                }
            }
        }

        let mut user_funcs_ir = String::new();
        let mut top_level_nodes = Vec::new();

        for node in ast {
            if matches!(node, ASTNode::FuncDecl { .. }) {
                let func_code = self.gen_func_decl(node)?;
                user_funcs_ir.push_str(&func_code);
                user_funcs_ir.push('\n');
            } else {
                top_level_nodes.push(node);
            }
        }

        self.entry_allocas.clear();
        let mut main_body = String::new();

        for node in top_level_nodes {
            let _ = self.gen_node(node, &mut main_body)?;
        }

        let mut main_ir = String::new();
        main_ir.push_str("define i32 @main() {\n");
        main_ir.push_str("entry:\n");
        main_ir.push_str(&self.entry_allocas);
        main_ir.push_str(&main_body);
        main_ir.push_str("  ret i32 0\n");
        main_ir.push_str("}\n");

        // Build header
        let mut header = String::new();
        header.push_str("; ModuleID = 'avelyn_module'\n");
        header.push_str("target datalayout = \"e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        header.push_str("target triple = \"x86_64-pc-windows-msvc\"\n\n");

        header.push_str("%SylvelVal = type { i32, i32, i64 }\n\n");

        // Declarations for C runtime functions
        header.push_str("declare void @sylvel_rt_make_null(%SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_make_bool(%SylvelVal*, i32)\n");
        header.push_str("declare void @sylvel_rt_make_int(%SylvelVal*, i64)\n");
        header.push_str("declare void @sylvel_rt_make_float(%SylvelVal*, double)\n");
        header.push_str("declare void @sylvel_rt_alloc_string(%SylvelVal*, i8*)\n");
        header.push_str("declare void @sylvel_rt_alloc_list(%SylvelVal*, i64)\n");
        header.push_str("declare void @sylvel_rt_alloc_map(%SylvelVal*, i64)\n");
        header.push_str("declare void @sylvel_rt_print(%SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_bin_op(%SylvelVal*, %SylvelVal*, i32, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_unary_op(%SylvelVal*, i32, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_list_push(%SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_list_get(%SylvelVal*, %SylvelVal*, i64)\n");
        header.push_str("declare void @sylvel_rt_list_set(%SylvelVal*, i64, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_map_get(%SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_map_set(%SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_subscript_get(%SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_subscript_set(%SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_call_expr(%SylvelVal*, %SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_builtin_assert(%SylvelVal*, %SylvelVal*, %SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_enter_try()\n");
        header.push_str("declare void @sylvel_rt_exit_try()\n");
        header.push_str("declare i32 @sylvel_rt_has_error()\n");
        header.push_str("declare void @sylvel_rt_clear_error()\n");
        header.push_str("declare i64 @sylvel_rt_len(%SylvelVal*)\n");
        header.push_str("declare i1 @sylvel_rt_to_bool(%SylvelVal*)\n");
        header.push_str("declare i64 @sylvel_rt_to_int(%SylvelVal*)\n");
        header.push_str("declare double @sylvel_rt_to_float(%SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_retain(%SylvelVal*)\n");
        header.push_str("declare void @sylvel_rt_release(%SylvelVal*)\n\n");

        // Declarations for dynamically discovered functions & builtins
        for (fn_name, arity) in &self.declared_func_signatures {
            let sig_args = vec!["%SylvelVal*".to_string(); arity + 1];
            header.push_str(&format!("declare void {}({})\n", fn_name, sig_args.join(", ")));
        }
        header.push('\n');

        // Emit string global constants
        for (label, content) in &self.str_constants {
            let escaped: String = content.bytes().map(|b| {
                if b == b'\n' { "\\0A".to_string() }
                else if b == b'\r' { "\\0D".to_string() }
                else if b == b'\t' { "\\09".to_string() }
                else if b == b'"' || b == b'\\' { format!("\\{:02X}", b) }
                else if b >= 32 && b <= 126 { (b as char).to_string() }
                else { format!("\\{:02X}", b) }
            }).collect();
            let len = content.as_bytes().len() + 1;
            header.push_str(&format!("{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n", label, len, escaped));
        }
        header.push('\n');

        let mut full_ir = header;
        full_ir.push_str(&user_funcs_ir);
        full_ir.push_str(&self.extra_user_funcs_ir.clone());
        full_ir.push_str(&main_ir);

        Ok(full_ir)
    }

    fn gen_func_decl(&mut self, node: &ASTNode) -> Result<String, String> {
        if let ASTNode::FuncDecl { name, params, body, .. } = node {
            self.push_scope();
            self.current_func_out_ptr = Some("%out".to_string());
            let saved_allocas = self.entry_allocas.clone();
            self.entry_allocas.clear();

            let mut param_sig = vec!["%SylvelVal* %out".to_string()];
            for (pname, _) in params {
                param_sig.push(format!("%SylvelVal* %param_{}", pname));
            }

            for (pname, default_opt) in params {
                self.set_var(pname, format!("%param_{}", pname));
            }

            let mut func_body = String::new();

            // Default parameter handling
            for (pname, default_opt) in params {
                if let Some(default_expr) = default_opt {
                    self.temp_count += 1;
                    let tag_temp = format!("%t{}", self.temp_count);
                    func_body.push_str(&format!("  {} = getelementptr inbounds %SylvelVal, %SylvelVal* %param_{}, i32 0, i32 0\n", tag_temp, pname));
                    self.temp_count += 1;
                    let tag_val = format!("%t{}", self.temp_count);
                    func_body.push_str(&format!("  {} = load i32, i32* {}\n", tag_val, tag_temp));
                    self.temp_count += 1;
                    let is_null = format!("%t{}", self.temp_count);
                    func_body.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_null, tag_val));

                    let set_def_lbl = self.new_label("def_set");
                    let skip_def_lbl = self.new_label("def_skip");
                    func_body.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_null, set_def_lbl, skip_def_lbl));

                    func_body.push_str(&format!("\n{}:\n", set_def_lbl));
                    let def_ptr = self.gen_node(default_expr, &mut func_body)?;
                    self.temp_count += 1;
                    let def_val = format!("%t{}", self.temp_count);
                    func_body.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", def_val, def_ptr));
                    func_body.push_str(&format!("  store %SylvelVal {}, %SylvelVal* %param_{}\n", def_val, pname));
                    func_body.push_str(&format!("  br label %{}\n", skip_def_lbl));

                    func_body.push_str(&format!("\n{}:\n", skip_def_lbl));
                }
            }

            let mut has_returned = false;
            for stmt in body {
                let _res_ptr = self.gen_node(stmt, &mut func_body)?;
                if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                    has_returned = true;
                    break;
                }
            }

            if !has_returned {
                func_body.push_str("  call void @sylvel_rt_make_null(%SylvelVal* %out)\n");
                func_body.push_str("  ret void\n");
            }

            let mut fn_ir = String::new();
            fn_ir.push_str(&format!("define void @lyn_fn_{}({}) {{\n", name, param_sig.join(", ")));
            fn_ir.push_str("entry:\n");
            fn_ir.push_str(&self.entry_allocas);
            fn_ir.push_str(&func_body);
            fn_ir.push_str("}\n");

            self.entry_allocas = saved_allocas;
            self.current_func_out_ptr = None;
            self.pop_scope();
            Ok(fn_ir)
        } else {
            Err("Expected FuncDecl".to_string())
        }
    }

    fn gen_node(&mut self, node: &ASTNode, out: &mut String) -> Result<String, String> {
        match node {
            ASTNode::Int(val) => {
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_int(%SylvelVal* {}, i64 {})\n", res_ptr, val));
                Ok(res_ptr)
            }
            ASTNode::Float(val) => {
                let res_ptr = self.new_temp_ptr(true);
                let f_str = if val.fract() == 0.0 {
                    format!("{:.1}", val)
                } else {
                    format!("{}", val)
                };
                out.push_str(&format!("  call void @sylvel_rt_make_float(%SylvelVal* {}, double {})\n", res_ptr, f_str));
                Ok(res_ptr)
            }
            ASTNode::Bool(val) => {
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_bool(%SylvelVal* {}, i32 {})\n", res_ptr, if *val { 1 } else { 0 }));
                Ok(res_ptr)
            }
            ASTNode::Null => {
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
            ASTNode::Str(val) => {
                let label = self.add_string_constant(val);
                self.temp_count += 1;
                let ptr_temp = format!("%t{}", self.temp_count);
                let len = val.as_bytes().len() + 1;
                out.push_str(&format!("  {} = getelementptr inbounds [{} x i8], [{} x i8]* {}, i64 0, i64 0\n", ptr_temp, len, len, label));
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_alloc_string(%SylvelVal* {}, i8* {})\n", res_ptr, ptr_temp));
                Ok(res_ptr)
            }
            ASTNode::ByteArray(bytes) => {
                let cap = bytes.len();
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_alloc_list(%SylvelVal* {}, i64 {})\n", res_ptr, cap));
                for b in bytes {
                    let b_ptr = self.new_temp_ptr(true);
                    out.push_str(&format!("  call void @sylvel_rt_make_int(%SylvelVal* {}, i64 {})\n", b_ptr, b));
                    out.push_str(&format!("  call void @sylvel_rt_list_push(%SylvelVal* {}, %SylvelVal* {})\n", res_ptr, b_ptr));
                }
                Ok(res_ptr)
            }
            ASTNode::Var(name) => {
                if let Some(var_ptr) = self.lookup_var(name) {
                    Ok(var_ptr)
                } else {
                    let res_ptr = self.new_temp_ptr(true);
                    out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                    Ok(res_ptr)
                }
            }
            ASTNode::Decl { name, value, .. } | ASTNode::Assign { name, value } => {
                let val_ptr = self.gen_node(value, out)?;
                let var_ptr = if let Some(p) = self.lookup_var(name) {
                    p
                } else {
                    let ptr = self.new_temp_ptr(true);
                    self.set_var(name, ptr.clone());
                    ptr
                };
                self.temp_count += 1;
                let v_load = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", v_load, val_ptr));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", v_load, var_ptr));
                Ok(var_ptr)
            }
            ASTNode::CompoundAssign { name, op, value } => {
                let bin_op_node = ASTNode::BinOp {
                    left: Box::new(ASTNode::Var(name.clone())),
                    op: op.clone(),
                    right: value.clone(),
                };
                let assign_node = ASTNode::Assign {
                    name: name.clone(),
                    value: Box::new(bin_op_node),
                };
                self.gen_node(&assign_node, out)
            }
            ASTNode::IndexAssign { target, index, value } => {
                let target_ptr = self.lookup_var(target).unwrap_or_else(|| self.new_temp_ptr(true));
                let index_ptr = self.gen_node(index, out)?;
                let val_ptr = self.gen_node(value, out)?;
                out.push_str(&format!("  call void @sylvel_rt_subscript_set(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", target_ptr, index_ptr, val_ptr));
                Ok(val_ptr)
            }
            ASTNode::PrintCall(expr) => {
                let val_ptr = self.gen_node(expr, out)?;
                out.push_str(&format!("  call void @sylvel_rt_print(%SylvelVal* {})\n", val_ptr));
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
            ASTNode::Assert { cond, msg } => {
                let cond_ptr = self.gen_node(cond, out)?;
                let msg_ptr = if let Some(m) = msg {
                    self.gen_node(m, out)?
                } else {
                    let ptr = self.new_temp_ptr(true);
                    out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", ptr));
                    ptr
                };
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_builtin_assert(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", res_ptr, cond_ptr, msg_ptr));
                Ok(res_ptr)
            }
            ASTNode::BinOp { left, op, right } => {
                let l_ptr = self.gen_node(left, out)?;
                let r_ptr = self.gen_node(right, out)?;
                let op_code = match op.as_str() {
                    "+" => 1, "-" => 2, "*" => 3, "/" => 4, "%" => 5,
                    "==" => 6, "!=" => 7, "<" => 8, "<=" => 9, ">" => 10, ">=" => 11,
                    "&" => 12, "|" => 13, "^" => 14, "<<" => 15, ">>" => 16,
                    "and" | "&&" => 17, "or" | "||" => 18,
                    _ => 1,
                };
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 {}, %SylvelVal* {})\n", res_ptr, l_ptr, op_code, r_ptr));
                Ok(res_ptr)
            }
            ASTNode::UnaryOp { op, operand } => {
                let val_ptr = self.gen_node(operand, out)?;
                let op_code = match op.as_str() {
                    "-" => 1,
                    "!" => 2,
                    _ => 1,
                };
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_unary_op(%SylvelVal* {}, i32 {}, %SylvelVal* {})\n", res_ptr, op_code, val_ptr));
                Ok(res_ptr)
            }
            ASTNode::Ternary { cond, then, els } => {
                let cond_ptr = self.gen_node(cond, out)?;
                self.temp_count += 1;
                let bool_temp = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", bool_temp, cond_ptr));

                let then_lbl = self.new_label("tern_then");
                let else_lbl = self.new_label("tern_else");
                let merge_lbl = self.new_label("tern_merge");

                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", bool_temp, then_lbl, else_lbl));

                let res_ptr = self.new_temp_ptr(true);

                out.push_str(&format!("\n{}:\n", then_lbl));
                let t_ptr = self.gen_node(then, out)?;
                self.temp_count += 1;
                let t_val = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", t_val, t_ptr));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", t_val, res_ptr));
                out.push_str(&format!("  br label %{}\n", merge_lbl));

                out.push_str(&format!("\n{}:\n", else_lbl));
                let e_ptr = self.gen_node(els, out)?;
                self.temp_count += 1;
                let e_val = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", e_val, e_ptr));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", e_val, res_ptr));
                out.push_str(&format!("  br label %{}\n", merge_lbl));

                out.push_str(&format!("\n{}:\n", merge_lbl));
                Ok(res_ptr)
            }
            ASTNode::NullCoalesce { left, right } => {
                let l_ptr = self.gen_node(left, out)?;
                self.temp_count += 1;
                let tag_temp = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = getelementptr inbounds %SylvelVal, %SylvelVal* {}, i32 0, i32 0\n", tag_temp, l_ptr));
                self.temp_count += 1;
                let tag_val = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load i32, i32* {}\n", tag_val, tag_temp));
                self.temp_count += 1;
                let is_null = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = icmp eq i32 {}, 0\n", is_null, tag_val));

                let null_lbl = self.new_label("nc_null");
                let nonnull_lbl = self.new_label("nc_nonnull");
                let merge_lbl = self.new_label("nc_merge");

                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_null, null_lbl, nonnull_lbl));

                let res_ptr = self.new_temp_ptr(true);

                out.push_str(&format!("\n{}:\n", null_lbl));
                let r_ptr = self.gen_node(right, out)?;
                self.temp_count += 1;
                let r_load = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", r_load, r_ptr));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", r_load, res_ptr));
                out.push_str(&format!("  br label %{}\n", merge_lbl));

                out.push_str(&format!("\n{}:\n", nonnull_lbl));
                self.temp_count += 1;
                let l_load = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", l_load, l_ptr));
                out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", l_load, res_ptr));
                out.push_str(&format!("  br label %{}\n", merge_lbl));

                out.push_str(&format!("\n{}:\n", merge_lbl));
                Ok(res_ptr)
            }
            ASTNode::Spread(expr) => {
                self.gen_node(expr, out)
            }
            ASTNode::Match { subject, arms } => {
                let subj_ptr = self.gen_node(subject, out)?;
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));

                let match_end_lbl = self.new_label("match_end");
                let mut next_check_lbl = self.new_label("match_check");

                out.push_str(&format!("  br label %{}\n", next_check_lbl));

                for (pattern, body) in arms {
                    let cur_check_lbl = next_check_lbl.clone();
                    next_check_lbl = self.new_label("match_check");
                    let hit_lbl = self.new_label("match_hit");

                    out.push_str(&format!("\n{}:\n", cur_check_lbl));

                    match pattern {
                        crate::ast::Pattern::Wildcard => {
                            out.push_str(&format!("  br label %{}\n", hit_lbl));
                        }
                        crate::ast::Pattern::Literal(lit_node) => {
                            if let ASTNode::BinOp { left: range_from, op, right: range_to } = lit_node {
                                if op == "..." || op == ".." {
                                    let ge_cmp = self.new_temp_ptr(true);
                                    let le_cmp = self.new_temp_ptr(true);
                                    let from_ptr = self.gen_node(range_from, out)?;
                                    let to_ptr = self.gen_node(range_to, out)?;

                                    out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 11, %SylvelVal* {})\n", ge_cmp, subj_ptr, from_ptr));
                                    out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 9, %SylvelVal* {})\n", le_cmp, subj_ptr, to_ptr));

                                    let cmp_ptr = self.new_temp_ptr(true);
                                    out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 17, %SylvelVal* {})\n", cmp_ptr, ge_cmp, le_cmp));

                                    self.temp_count += 1;
                                    let is_match = format!("%t{}", self.temp_count);
                                    out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", is_match, cmp_ptr));
                                    out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_match, hit_lbl, next_check_lbl));
                                } else {
                                    let lit_ptr = self.gen_node(lit_node, out)?;
                                    let cmp_ptr = self.new_temp_ptr(true);
                                    out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 6, %SylvelVal* {})\n", cmp_ptr, subj_ptr, lit_ptr));

                                    self.temp_count += 1;
                                    let is_match = format!("%t{}", self.temp_count);
                                    out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", is_match, cmp_ptr));
                                    out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_match, hit_lbl, next_check_lbl));
                                }
                            } else {
                                let lit_ptr = self.gen_node(lit_node, out)?;
                                let cmp_ptr = self.new_temp_ptr(true);
                                out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 6, %SylvelVal* {})\n", cmp_ptr, subj_ptr, lit_ptr));

                                self.temp_count += 1;
                                let is_match = format!("%t{}", self.temp_count);
                                out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", is_match, cmp_ptr));
                                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_match, hit_lbl, next_check_lbl));
                            }
                        }
                        _ => {
                            out.push_str(&format!("  br label %{}\n", hit_lbl));
                        }
                    }

                    out.push_str(&format!("\n{}:\n", hit_lbl));
                    self.push_scope();
                    let mut arm_res = self.new_temp_ptr(true);
                    for stmt in body {
                        arm_res = self.gen_node(stmt, out)?;
                    }
                    self.temp_count += 1;
                    let arm_val = format!("%t{}", self.temp_count);
                    out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", arm_val, arm_res));
                    out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", arm_val, res_ptr));
                    self.pop_scope();
                    out.push_str(&format!("  br label %{}\n", match_end_lbl));
                }

                out.push_str(&format!("\n{}:\n", next_check_lbl));
                out.push_str(&format!("  br label %{}\n", match_end_lbl));

                out.push_str(&format!("\n{}:\n", match_end_lbl));
                Ok(res_ptr)
            }
            ASTNode::If { cond, then, els } => {
                let cond_ptr = self.gen_node(cond, out)?;
                self.temp_count += 1;
                let bool_temp = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", bool_temp, cond_ptr));

                let then_lbl = self.new_label("then");
                let else_lbl = self.new_label("else");
                let merge_lbl = self.new_label("merge");

                let has_else = els.is_some();
                let false_lbl = if has_else { &else_lbl } else { &merge_lbl };

                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", bool_temp, then_lbl, false_lbl));

                // Then block
                out.push_str(&format!("\n{}:\n", then_lbl));
                self.push_scope();
                let mut last_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", last_ptr));
                let mut then_terminated = false;
                for stmt in then {
                    last_ptr = self.gen_node(stmt, out)?;
                    if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                        then_terminated = true;
                        break;
                    }
                }
                self.pop_scope();
                if !then_terminated {
                    out.push_str(&format!("  br label %{}\n", merge_lbl));
                }

                // Else block
                let mut else_terminated = false;
                if let Some(else_stmts) = els {
                    out.push_str(&format!("\n{}:\n", else_lbl));
                    self.push_scope();
                    for stmt in else_stmts {
                        last_ptr = self.gen_node(stmt, out)?;
                        if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                            else_terminated = true;
                            break;
                        }
                    }
                    self.pop_scope();
                    if !else_terminated {
                        out.push_str(&format!("  br label %{}\n", merge_lbl));
                    }
                }

                let need_merge = !has_else || !then_terminated || !else_terminated;
                if need_merge {
                    out.push_str(&format!("\n{}:\n", merge_lbl));
                }

                Ok(last_ptr)
            }
            ASTNode::While { cond, body } => {
                let cond_lbl = self.new_label("while_cond");
                let body_lbl = self.new_label("while_body");
                let exit_lbl = self.new_label("while_exit");

                self.current_loop_labels.push((exit_lbl.clone(), cond_lbl.clone()));
                out.push_str(&format!("  br label %{}\n", cond_lbl));

                out.push_str(&format!("\n{}:\n", cond_lbl));
                let cond_ptr = self.gen_node(cond, out)?;
                self.temp_count += 1;
                let bool_temp = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", bool_temp, cond_ptr));
                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", bool_temp, body_lbl, exit_lbl));

                out.push_str(&format!("\n{}:\n", body_lbl));
                self.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.gen_node(stmt, out)?;
                    if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                        body_terminated = true;
                        break;
                    }
                }
                self.pop_scope();
                if !body_terminated {
                    out.push_str(&format!("  br label %{}\n", cond_lbl));
                }

                out.push_str(&format!("\n{}:\n", exit_lbl));
                self.current_loop_labels.pop();

                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
            ASTNode::ForRange { var, from, to, body, inclusive } => {
                let from_ptr = self.gen_node(from, out)?;
                let to_ptr = self.gen_node(to, out)?;

                self.temp_count += 1;
                let start_idx = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i64 @sylvel_rt_to_int(%SylvelVal* {})\n", start_idx, from_ptr));

                self.temp_count += 1;
                let end_idx = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i64 @sylvel_rt_to_int(%SylvelVal* {})\n", end_idx, to_ptr));

                let idx_ptr = self.new_temp_ptr(false);
                out.push_str(&format!("  store i64 {}, i64* {}\n", start_idx, idx_ptr));

                let var_val_ptr = if let Some(p) = self.lookup_var(var) {
                    p
                } else {
                    let ptr = self.new_temp_ptr(true);
                    self.set_var(var, ptr.clone());
                    ptr
                };

                let cond_lbl = self.new_label("for_cond");
                let body_lbl = self.new_label("for_body");
                let step_lbl = self.new_label("for_step");
                let exit_lbl = self.new_label("for_exit");

                self.current_loop_labels.push((exit_lbl.clone(), step_lbl.clone()));
                out.push_str(&format!("  br label %{}\n", cond_lbl));

                out.push_str(&format!("\n{}:\n", cond_lbl));
                self.temp_count += 1;
                let curr_i = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load i64, i64* {}\n", curr_i, idx_ptr));

                self.temp_count += 1;
                let cmp_res = format!("%t{}", self.temp_count);
                if *inclusive {
                    out.push_str(&format!("  {} = icmp sle i64 {}, {}\n", cmp_res, curr_i, end_idx));
                } else {
                    out.push_str(&format!("  {} = icmp slt i64 {}, {}\n", cmp_res, curr_i, end_idx));
                }
                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_res, body_lbl, exit_lbl));

                out.push_str(&format!("\n{}:\n", body_lbl));
                out.push_str(&format!("  call void @sylvel_rt_make_int(%SylvelVal* {}, i64 {})\n", var_val_ptr, curr_i));

                self.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.gen_node(stmt, out)?;
                    if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                        body_terminated = true;
                        break;
                    }
                }
                self.pop_scope();
                if !body_terminated {
                    out.push_str(&format!("  br label %{}\n", step_lbl));
                }

                out.push_str(&format!("\n{}:\n", step_lbl));
                self.temp_count += 1;
                let curr_i_step = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load i64, i64* {}\n", curr_i_step, idx_ptr));

                self.temp_count += 1;
                let next_i = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = add i64 {}, 1\n", next_i, curr_i_step));
                out.push_str(&format!("  store i64 {}, i64* {}\n", next_i, idx_ptr));
                out.push_str(&format!("  br label %{}\n", cond_lbl));

                out.push_str(&format!("\n{}:\n", exit_lbl));
                self.current_loop_labels.pop();

                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
            ASTNode::For { var, iter, body } => {
                let iter_ptr = self.gen_node(iter, out)?;

                self.temp_count += 1;
                let len_val = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i64 @sylvel_rt_len(%SylvelVal* {})\n", len_val, iter_ptr));

                let idx_ptr = self.new_temp_ptr(false);
                out.push_str(&format!("  store i64 0, i64* {}\n", idx_ptr));

                let var_val_ptr = if let Some(p) = self.lookup_var(var) {
                    p
                } else {
                    let ptr = self.new_temp_ptr(true);
                    self.set_var(var, ptr.clone());
                    ptr
                };

                let cond_lbl = self.new_label("for_iter_cond");
                let body_lbl = self.new_label("for_iter_body");
                let step_lbl = self.new_label("for_iter_step");
                let exit_lbl = self.new_label("for_iter_exit");

                self.current_loop_labels.push((exit_lbl.clone(), step_lbl.clone()));
                out.push_str(&format!("  br label %{}\n", cond_lbl));

                out.push_str(&format!("\n{}:\n", cond_lbl));
                self.temp_count += 1;
                let curr_i = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load i64, i64* {}\n", curr_i, idx_ptr));

                self.temp_count += 1;
                let cmp_res = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = icmp slt i64 {}, {}\n", cmp_res, curr_i, len_val));
                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_res, body_lbl, exit_lbl));

                out.push_str(&format!("\n{}:\n", body_lbl));
                let idx_val_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_int(%SylvelVal* {}, i64 {})\n", idx_val_ptr, curr_i));
                out.push_str(&format!("  call void @sylvel_rt_subscript_get(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", var_val_ptr, iter_ptr, idx_val_ptr));

                self.push_scope();
                let mut body_terminated = false;
                for stmt in body {
                    let _ = self.gen_node(stmt, out)?;
                    if matches!(stmt, ASTNode::Return(_) | ASTNode::Break | ASTNode::Continue) {
                        body_terminated = true;
                        break;
                    }
                }
                self.pop_scope();
                if !body_terminated {
                    out.push_str(&format!("  br label %{}\n", step_lbl));
                }

                out.push_str(&format!("\n{}:\n", step_lbl));
                self.temp_count += 1;
                let curr_i_step = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = load i64, i64* {}\n", curr_i_step, idx_ptr));

                self.temp_count += 1;
                let next_i = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = add i64 {}, 1\n", next_i, curr_i_step));
                out.push_str(&format!("  store i64 {}, i64* {}\n", next_i, idx_ptr));
                out.push_str(&format!("  br label %{}\n", cond_lbl));

                out.push_str(&format!("\n{}:\n", exit_lbl));
                self.current_loop_labels.pop();

                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
            ASTNode::Lambda { params, body, .. } => {
                let lambda_name = format!("__lambda_{}", self.lambda_count);
                self.lambda_count += 1;

                let decl_node = ASTNode::FuncDecl {
                    name: lambda_name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    variadic: false,
                    annotations: Vec::new(),
                };
                self.user_func_names.insert(lambda_name.clone());
                let fn_ir = self.gen_func_decl(&decl_node)?;
                self.extra_user_funcs_ir.push_str(&fn_ir);

                let label = self.add_string_constant(&lambda_name);
                self.temp_count += 1;
                let ptr_temp = format!("%t{}", self.temp_count);
                let len = lambda_name.as_bytes().len() + 1;
                out.push_str(&format!("  {} = getelementptr inbounds [{} x i8], [{} x i8]* {}, i64 0, i64 0\n", ptr_temp, len, len, label));
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_alloc_string(%SylvelVal* {}, i8* {})\n", res_ptr, ptr_temp));
                Ok(res_ptr)
            }
            ASTNode::FuncCall { name, args } => {
                let res_ptr = self.new_temp_ptr(true);

                let mut arg_temps = vec![format!("%SylvelVal* {}", res_ptr)];
                for arg in args {
                    let a_ptr = self.gen_node(arg, out)?;
                    arg_temps.push(format!("%SylvelVal* {}", a_ptr));
                }

                let target_fn = if self.user_func_names.contains(name) {
                    format!("@lyn_fn_{}", name)
                } else {
                    let fn_name = format!("@sylvel_rt_builtin_{}", name);
                    let arg_cnt = args.len();
                    self.declared_func_signatures.entry(fn_name.clone()).or_insert(arg_cnt);
                    fn_name
                };

                out.push_str(&format!("  call void {}({})\n", target_fn, arg_temps.join(", ")));
                Ok(res_ptr)
            }
            ASTNode::CallExpr { callee, args } => {
                let callee_ptr = self.gen_node(callee, out)?;
                let arg1_ptr = if args.len() > 0 { self.gen_node(&args[0], out)? } else { self.new_temp_ptr(true) };
                if args.len() == 0 {
                    out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", arg1_ptr));
                }
                let arg2_ptr = if args.len() > 1 { self.gen_node(&args[1], out)? } else { self.new_temp_ptr(true) };
                if args.len() <= 1 {
                    out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", arg2_ptr));
                }

                let res_ptr = self.new_temp_ptr(true);

                let user_funcs: Vec<String> = self.user_func_names.iter().cloned().collect();
                let end_lbl = self.new_label("call_expr_end");
                let mut next_lbl = self.new_label("call_user_check");

                for fn_name in &user_funcs {
                    let cur_lbl = next_lbl.clone();
                    next_lbl = self.new_label("call_user_check");
                    let hit_lbl = self.new_label("call_user_hit");

                    let fn_name_lbl = self.add_string_constant(fn_name);
                    self.temp_count += 1;
                    let fn_name_str_ptr = format!("%t{}", self.temp_count);
                    let fn_len = fn_name.as_bytes().len() + 1;
                    out.push_str(&format!("  {} = getelementptr inbounds [{} x i8], [{} x i8]* {}, i64 0, i64 0\n", fn_name_str_ptr, fn_len, fn_len, fn_name_lbl));

                    let match_val = self.new_temp_ptr(true);
                    out.push_str(&format!("  call void @sylvel_rt_alloc_string(%SylvelVal* {}, i8* {})\n", match_val, fn_name_str_ptr));

                    let cmp_val = self.new_temp_ptr(true);
                    out.push_str(&format!("  call void @sylvel_rt_bin_op(%SylvelVal* {}, %SylvelVal* {}, i32 6, %SylvelVal* {})\n", cmp_val, callee_ptr, match_val));

                    self.temp_count += 1;
                    let is_hit = format!("%t{}", self.temp_count);
                    out.push_str(&format!("  {} = call i1 @sylvel_rt_to_bool(%SylvelVal* {})\n", is_hit, cmp_val));
                    out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_hit, hit_lbl, cur_lbl));

                    out.push_str(&format!("\n{}:\n", hit_lbl));
                    out.push_str(&format!("  call void @lyn_fn_{}(%SylvelVal* {}, %SylvelVal* {})\n", fn_name, res_ptr, arg1_ptr));
                    out.push_str(&format!("  br label %{}\n", end_lbl));

                    out.push_str(&format!("\n{}:\n", cur_lbl));
                }

                out.push_str(&format!("  call void @sylvel_rt_call_expr(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", res_ptr, callee_ptr, arg1_ptr, arg2_ptr));
                out.push_str(&format!("  br label %{}\n", end_lbl));

                out.push_str(&format!("\n{}:\n", end_lbl));

                Ok(res_ptr)
            }
            ASTNode::Return(expr) => {
                let val_ptr = self.gen_node(expr, out)?;
                if let Some(out_ptr) = self.current_func_out_ptr.clone() {
                    self.temp_count += 1;
                    let v_load = format!("%t{}", self.temp_count);
                    out.push_str(&format!("  {} = load %SylvelVal, %SylvelVal* {}\n", v_load, val_ptr));
                    out.push_str(&format!("  store %SylvelVal {}, %SylvelVal* {}\n", v_load, out_ptr));
                    out.push_str("  ret void\n");
                }
                Ok(val_ptr)
            }
            ASTNode::Break => {
                if let Some((exit_lbl, _)) = self.current_loop_labels.last() {
                    out.push_str(&format!("  br label %{}\n", exit_lbl));
                }
                let res_ptr = self.new_temp_ptr(true);
                Ok(res_ptr)
            }
            ASTNode::Continue => {
                if let Some((_, step_lbl)) = self.current_loop_labels.last() {
                    out.push_str(&format!("  br label %{}\n", step_lbl));
                }
                let res_ptr = self.new_temp_ptr(true);
                Ok(res_ptr)
            }
            ASTNode::ArrayLit(items) => {
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_alloc_list(%SylvelVal* {}, i64 {})\n", res_ptr, items.len()));
                for item in items {
                    if let ASTNode::Spread(inner) = item {
                        let inner_ptr = self.gen_node(inner, out)?;
                        self.temp_count += 1;
                        let len_val = format!("%t{}", self.temp_count);
                        out.push_str(&format!("  {} = call i64 @sylvel_rt_len(%SylvelVal* {})\n", len_val, inner_ptr));

                        let idx_ptr = self.new_temp_ptr(false);
                        out.push_str(&format!("  store i64 0, i64* {}\n", idx_ptr));

                        let cond_lbl = self.new_label("spread_cond");
                        let body_lbl = self.new_label("spread_body");
                        let exit_lbl = self.new_label("spread_exit");

                        out.push_str(&format!("  br label %{}\n", cond_lbl));

                        out.push_str(&format!("\n{}:\n", cond_lbl));
                        self.temp_count += 1;
                        let curr_i = format!("%t{}", self.temp_count);
                        out.push_str(&format!("  {} = load i64, i64* {}\n", curr_i, idx_ptr));

                        self.temp_count += 1;
                        let cmp_res = format!("%t{}", self.temp_count);
                        out.push_str(&format!("  {} = icmp slt i64 {}, {}\n", cmp_res, curr_i, len_val));
                        out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_res, body_lbl, exit_lbl));

                        out.push_str(&format!("\n{}:\n", body_lbl));
                        let item_val_ptr = self.new_temp_ptr(true);
                        out.push_str(&format!("  call void @sylvel_rt_list_get(%SylvelVal* {}, %SylvelVal* {}, i64 {})\n", item_val_ptr, inner_ptr, curr_i));
                        out.push_str(&format!("  call void @sylvel_rt_list_push(%SylvelVal* {}, %SylvelVal* {})\n", res_ptr, item_val_ptr));

                        self.temp_count += 1;
                        let next_i = format!("%t{}", self.temp_count);
                        out.push_str(&format!("  {} = add i64 {}, 1\n", next_i, curr_i));
                        out.push_str(&format!("  store i64 {}, i64* {}\n", next_i, idx_ptr));
                        out.push_str(&format!("  br label %{}\n", cond_lbl));

                        out.push_str(&format!("\n{}:\n", exit_lbl));
                    } else {
                        let item_ptr = self.gen_node(item, out)?;
                        out.push_str(&format!("  call void @sylvel_rt_list_push(%SylvelVal* {}, %SylvelVal* {})\n", res_ptr, item_ptr));
                    }
                }
                Ok(res_ptr)
            }
            ASTNode::MapLit(pairs) => {
                let cap = pairs.len();
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_alloc_map(%SylvelVal* {}, i64 {})\n", res_ptr, cap));
                for (k, v) in pairs {
                    let k_ptr = self.gen_node(k, out)?;
                    let v_ptr = self.gen_node(v, out)?;
                    out.push_str(&format!("  call void @sylvel_rt_map_set(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", res_ptr, k_ptr, v_ptr));
                }
                Ok(res_ptr)
            }
            ASTNode::Subscript { target, index } => {
                let target_ptr = self.gen_node(target, out)?;
                let index_ptr = self.gen_node(index, out)?;
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_subscript_get(%SylvelVal* {}, %SylvelVal* {}, %SylvelVal* {})\n", res_ptr, target_ptr, index_ptr));
                Ok(res_ptr)
            }
            ASTNode::TryCatch { body, catches, finally_body } => {
                out.push_str("  call void @sylvel_rt_enter_try()\n");
                self.push_scope();
                let mut last_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", last_ptr));

                let catch_lbl = self.new_label("try_catch");
                let normal_lbl = self.new_label("try_normal");
                let end_lbl = self.new_label("try_end");

                for stmt in body {
                    last_ptr = self.gen_node(stmt, out)?;
                }

                self.temp_count += 1;
                let has_err = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = call i32 @sylvel_rt_has_error()\n", has_err));
                self.temp_count += 1;
                let is_err = format!("%t{}", self.temp_count);
                out.push_str(&format!("  {} = icmp ne i32 {}, 0\n", is_err, has_err));
                out.push_str(&format!("  br i1 {}, label %{}, label %{}\n", is_err, catch_lbl, normal_lbl));

                out.push_str(&format!("\n{}:\n", normal_lbl));
                out.push_str("  call void @sylvel_rt_exit_try()\n");
                out.push_str(&format!("  br label %{}\n", end_lbl));

                out.push_str(&format!("\n{}:\n", catch_lbl));
                out.push_str("  call void @sylvel_rt_exit_try()\n");
                out.push_str("  call void @sylvel_rt_clear_error()\n");
                if let Some((_, var_name, catch_stmts)) = catches.first() {
                    let err_val_ptr = self.new_temp_ptr(true);
                    let label = self.add_string_constant("caught");
                    self.temp_count += 1;
                    let ptr_temp = format!("%t{}", self.temp_count);
                    out.push_str(&format!("  {} = getelementptr inbounds [7 x i8], [7 x i8]* {}, i64 0, i64 0\n", ptr_temp, label));
                    out.push_str(&format!("  call void @sylvel_rt_alloc_string(%SylvelVal* {}, i8* {})\n", err_val_ptr, ptr_temp));
                    self.set_var(var_name, err_val_ptr);
                    for stmt in catch_stmts {
                        last_ptr = self.gen_node(stmt, out)?;
                    }
                }
                out.push_str(&format!("  br label %{}\n", end_lbl));

                out.push_str(&format!("\n{}:\n", end_lbl));
                self.pop_scope();

                if let Some(fin) = finally_body {
                    for stmt in fin {
                        let _ = self.gen_node(&stmt, out)?;
                    }
                }
                Ok(last_ptr)
            }
            ASTNode::Throw(expr) => {
                let res_ptr = self.gen_node(expr, out)?;
                Ok(res_ptr)
            }
            _ => {
                let res_ptr = self.new_temp_ptr(true);
                out.push_str(&format!("  call void @sylvel_rt_make_null(%SylvelVal* {})\n", res_ptr));
                Ok(res_ptr)
            }
        }
    }
}
