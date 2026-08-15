// pipeline_tests.rs — Comprehensive unit and stage tests for the AIR compiler pipeline

#[cfg(test)]
mod pipeline_unit_tests {
    use crate::ast::{ASTNode, AvelynType, Span, TypedNode};
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::sema::{DiagnosticEmitter, SemaContext, NameResolver, TypeChecker, SymbolKind};
    use crate::air::{
        AirFunction, AirModule, AirType, BlockId, Inst, RuntimeFn, Value, VOID_VALUE,
        verify::verify_module, printer::AirPrinter,
    };
    use crate::airgen::lower_to_air;
    use crate::optimizer::{optimize, OptLevel};
    use crate::irgen::lower_to_llvm;
    use crate::target::Target;

    // Helper: Parse a string into AST
    fn parse_src(src: &str) -> Vec<ASTNode> {
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    // ─── 1. AST & Type Tests ──────────────────────────────────────────────────
    #[test]
    fn test_ast_span_and_types() {
        let span = Span::new(1, 10, 5);
        assert_eq!(span.to_string(), "10:5");
        assert!(span.is_known());

        let unknown = Span::UNKNOWN;
        assert!(!unknown.is_known());

        let int_ty = AvelynType::Int;
        assert!(int_ty.is_scalar());
        assert!(!int_ty.is_heap());

        let str_ty = AvelynType::Str;
        assert!(!str_ty.is_scalar());
        assert!(str_ty.is_heap());

        let list_ty = AvelynType::List(Box::new(AvelynType::Int));
        assert_eq!(list_ty.to_string(), "list[int]");
    }

    // ─── 2. Sema & Diagnostics Tests ──────────────────────────────────────────
    #[test]
    fn test_sema_name_resolution_and_type_check() {
        let code = r#"
fn add(a, b)
    return a + b

let x = 10
let y = 20
let z = add(x, y)
"#;
        let ast = parse_src(code);
        let mut sema = SemaContext::new();
        let typed = sema.analyse(&ast);

        assert!(!sema.diag.has_errors());
        assert_eq!(typed.len(), ast.len());
    }

    #[test]
    fn test_sema_undeclared_warning() {
        let code = r#"
let a = undefined_variable_name + 1
"#;
        let ast = parse_src(code);
        let mut sema = SemaContext::new();
        let _ = sema.analyse(&ast);

        assert!(sema.diag.warning_count() > 0);
        assert!(!sema.diag.has_errors());
    }

    // ─── 3. AIRGen Tests ──────────────────────────────────────────────────────
    #[test]
    fn test_airgen_basic_function_and_main() {
        let code = r#"
fn multiply(x, y)
    return x * y

let res = multiply(3, 4)
"#;
        let ast = parse_src(code);
        let mut diag = DiagnosticEmitter::new();
        let air = lower_to_air(&ast, &mut diag).expect("AIRGen failed");

        assert!(!diag.has_errors());
        assert!(air.functions.len() >= 2); // multiply and main

        let fn_names: Vec<_> = air.functions.iter().map(|f| &f.name).collect();
        assert!(fn_names.iter().any(|n| n.contains("multiply")));
        assert!(fn_names.iter().any(|n| *n == "main"));
    }

    #[test]
    fn test_airgen_control_flow_lowering() {
        let code = r#"
let i = 0
while i < 10
    if i == 5
        print("five")
    i = i + 1
"#;
        let ast = parse_src(code);
        let mut diag = DiagnosticEmitter::new();
        let air = lower_to_air(&ast, &mut diag).expect("AIRGen failed");

        assert!(!diag.has_errors());
        let main_fn = air.function_by_name("main").expect("main function missing");
        // Ensure multiple basic blocks are created for while and if
        assert!(main_fn.blocks.len() > 3);
    }

    // ─── 4. Verifier Tests ────────────────────────────────────────────────────
    #[test]
    fn test_air_verifier_valid_module() {
        let code = r#"
let a = 1
let b = 2
let c = a + b
"#;
        let ast = parse_src(code);
        let mut diag = DiagnosticEmitter::new();
        let air = lower_to_air(&ast, &mut diag).expect("AIRGen failed");

        let errs = verify_module(&air);
        assert!(errs.is_empty(), "Verifier reported errors: {:?}", errs);
    }

    #[test]
    fn test_air_verifier_catches_invalid_block() {
        let mut module = AirModule::new("test_invalid");
        let mut func = AirFunction::new("bad_fn", Span::UNKNOWN);
        let b0 = func.fresh_block("entry");
        // No terminator added to block b0
        func.push_to(b0, Inst::ConstInt(Value(1), 42));
        module.add_function(func);

        let errs = verify_module(&module);
        assert!(!errs.is_empty(), "Verifier should catch unterminated block");
    }

    // ─── 5. Optimizer Tests ───────────────────────────────────────────────────
    #[test]
    fn test_optimizer_const_fold() {
        let mut module = AirModule::new("test_opt");
        let mut func = AirFunction::new("main", Span::UNKNOWN);
        let b0 = func.fresh_block("entry");

        let v1 = Value(1);
        let v2 = Value(2);
        let v3 = Value(3);
        let v_out = Value(0);

        func.push_to(b0, Inst::ConstInt(v1, 10));
        func.push_to(b0, Inst::ConstInt(v2, 25));
        func.push_to(b0, Inst::IAdd(v3, v1, v2)); // 10 + 25 = 35
        func.push_to(b0, Inst::Return(v3));
        func.rebuild_cfg();

        module.add_function(func);

        let passes_run = optimize(&mut module, OptLevel::O2);
        assert!(passes_run > 0);

        let opt_fn = &module.functions[0];
        let has_folded = opt_fn.blocks[0].insts.iter().any(|inst| match inst {
            Inst::ConstInt(v, val) => *v == v3 && *val == 35,
            _ => false,
        });
        assert!(has_folded, "Constant folding should have produced ConstInt(35)");
    }

    #[test]
    fn test_optimizer_unreachable_and_dce() {
        let mut module = AirModule::new("test_dce");
        let mut func = AirFunction::new("main", Span::UNKNOWN);
        let b0 = func.fresh_block("entry");
        let b_dead = func.fresh_block("unreachable_block");

        let v1 = Value(1);
        let v_unused = Value(2);

        // b0 returns unconditionally
        func.push_to(b0, Inst::ConstInt(v1, 0));
        func.push_to(b0, Inst::Return(v1));

        // b_dead is never jumped to
        func.push_to(b_dead, Inst::ConstInt(v_unused, 999));
        func.push_to(b_dead, Inst::Return(v_unused));

        func.rebuild_cfg();
        module.add_function(func);

        optimize(&mut module, OptLevel::O1);

        let opt_fn = &module.functions[0];
        assert_eq!(opt_fn.blocks.len(), 1, "Unreachable block should be eliminated");
    }

    // ─── 6. LLVM IRGen Tests ──────────────────────────────────────────────────
    #[test]
    fn test_irgen_emits_valid_llvm_structure() {
        let code = r#"
fn greet(name)
    print("Hello " + name)

greet("World")
"#;
        let ast = parse_src(code);
        let mut diag = DiagnosticEmitter::new();
        let mut air = lower_to_air(&ast, &mut diag).expect("AIRGen failed");
        optimize(&mut air, OptLevel::O2);

        let target = Target::host_default();
        let llvm_ir = lower_to_llvm(&air, &target);

        assert!(llvm_ir.contains("target datalayout ="));
        assert!(llvm_ir.contains("target triple ="));
        assert!(llvm_ir.contains("%SylvelVal = type { i32, i32, i64 }"));
        assert!(llvm_ir.contains("define i32 @main()"));
        assert!(llvm_ir.contains("sylvel_rt_"));
    }

    // ─── 7. End-to-End Compiler Pipeline Test ────────────────────────────────
    #[test]
    fn test_end_to_end_air_pipeline() {
        let code = r#"
fn factorial(n)
    if n <= 1
        return 1
    return n * factorial(n - 1)

let ans = factorial(5)
print(ans)
"#;
        let ast = parse_src(code);

        // 1. Sema
        let mut sema = SemaContext::new();
        let _typed = sema.analyse(&ast);
        assert!(!sema.diag.has_errors());

        // 2. AIRGen
        let mut diag = DiagnosticEmitter::new();
        let mut air = lower_to_air(&ast, &mut diag).expect("AIRGen failed");
        assert!(!diag.has_errors());

        // 3. Verify
        let _ = verify_module(&air);

        // 4. Optimize
        optimize(&mut air, OptLevel::O2);

        // 5. Emit AIR text
        let air_text = AirPrinter::print_module(&air);
        assert!(air_text.contains("fn @lyn_fn_factorial"));
        assert!(air_text.contains("fn @main"));

        // 6. IRGen
        let target = Target::host_default();
        let llvm_ir = lower_to_llvm(&air, &target);
        assert!(llvm_ir.contains("@lyn_fn_factorial"));
    }
}
