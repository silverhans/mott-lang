//! Semantic analysis: walks the AST after parsing, validates types and
//! scopes, surfaces all language-rule errors (mismatches, undefined
//! symbols, `sac` outside a loop, etc.).
//!
//! Used to live mixed into the C backend — emitting and validating in the
//! same pass. That worked for v0.1 but the cost of staying in one pass got
//! high once arrays and dynamic operations landed. Splitting now means:
//!   - codegen becomes a near-mechanical walk over a known-valid AST,
//!   - sema is the one place to look when reasoning about typing rules,
//!   - future fronts (LLVM, WASM) re-use sema without copying its logic.
//!
//! Sema doesn't mutate the AST — types are inferred during the check and
//! used immediately. Codegen still does its own light type inference for
//! monomorphic dispatch (`mott_yazde_terah` vs `_deshnash`); we accept
//! that small duplication until a typed-AST refactor is worth doing.
//!
//! Errors carry no source positions yet. Adding them means plumbing
//! line/col onto every AST node — easy mechanical change when it starts
//! to bite (currently sema errors are rare enough that "type mismatch:
//! `name` declared as terah but initializer is bool" finds the bug).

use std::collections::{HashMap, HashSet};

use crate::ast::{
    BinOp, Block, Expr, Function, Item, IterSource, Program, Stmt, StringPart, Type, UnOp,
};
use crate::error::{Error, Result};

/// Function signature info collected up-front so call sites can be
/// validated regardless of declaration order.
#[derive(Debug, Clone)]
pub struct FuncSig {
    pub params: Vec<Type>,
    pub return_type: Option<Type>,
}

/// Output of sema, consumed by the backend. Right now it's just function
/// signatures (used for Call type dispatch); the backend re-infers
/// expression types itself. When we move to a typed AST this will grow
/// an `expr_types: HashMap<...>` field.
#[derive(Debug)]
pub struct TypeInfo {
    pub functions: HashMap<String, FuncSig>,
}

/// Run semantic analysis over a program. Returns `TypeInfo` for the
/// backend, or the first error encountered.
pub fn check(program: &Program) -> Result<TypeInfo> {
    let mut checker = Checker::new(program)?;
    checker.check_program(program)?;
    Ok(TypeInfo {
        functions: checker.functions,
    })
}

struct Checker {
    /// Stack of scopes. Outermost is the function's scope (parameters);
    /// each `{ ... }` body and each `yallalc` body adds a frame.
    scopes: Vec<HashMap<String, Type>>,
    /// All declared functions, keyed by name. Pre-populated before
    /// checking bodies so calls in any direction resolve.
    functions: HashMap<String, FuncSig>,
    /// Depth of enclosing `cqachunna` / `yallalc` loops. Validates
    /// `sac` and `khida` only appear inside one.
    loop_depth: usize,
    /// Names of the current function's parameters. push/pop reject on
    /// these because Mott arrays are value types sharing a buffer — a
    /// realloc in push would invalidate the caller's pointer.
    current_params: HashSet<String>,
    /// Return type of the function we're currently checking. None for
    /// void functions. Used to validate `yuxadalo expr` matches.
    current_return: Option<Type>,
}

impl Checker {
    fn new(program: &Program) -> Result<Self> {
        // Collect all function signatures up front so call sites can
        // resolve even before the callee is defined in source order.
        let mut functions = HashMap::new();
        for item in &program.items {
            let Item::Function(f) = item;
            let sig = FuncSig {
                params: f.params.iter().map(|p| p.ty.clone()).collect(),
                return_type: f.return_type.clone(),
            };
            if functions.insert(f.name.clone(), sig).is_some() {
                return Err(Error::Sema(format!("duplicate function `{}`", f.name)));
            }
        }
        Ok(Self {
            scopes: Vec::new(),
            functions,
            loop_depth: 0,
            current_params: HashSet::new(),
            current_return: None,
        })
    }

    fn check_program(&mut self, program: &Program) -> Result<()> {
        for item in &program.items {
            let Item::Function(f) = item;
            self.check_function(f)?;
        }
        Ok(())
    }

    fn check_function(&mut self, f: &Function) -> Result<()> {
        // The entry point has fixed shape constraints — enforced here
        // rather than in the parser so we can match the signature in
        // codegen later (`int main(void)`).
        if f.name == "kort" {
            if !f.params.is_empty() {
                return Err(Error::Sema(
                    "entry function `kort` must not take parameters".into(),
                ));
            }
            if f.return_type.is_some() {
                return Err(Error::Sema(
                    "entry function `kort` must not declare a return type".into(),
                ));
            }
        }

        self.push_scope();
        self.current_params = f.params.iter().map(|p| p.name.clone()).collect();
        self.current_return = f.return_type.clone();
        for p in &f.params {
            self.declare(&p.name, p.ty.clone());
        }
        for s in &f.body.stmts {
            self.check_stmt(s)?;
        }
        self.pop_scope();
        self.current_params.clear();
        self.current_return = None;
        Ok(())
    }

    fn check_block(&mut self, b: &Block) -> Result<()> {
        self.push_scope();
        let result = (|| {
            for s in &b.stmts {
                self.check_stmt(s)?;
            }
            Ok(())
        })();
        self.pop_scope();
        result
    }

    fn check_stmt(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let { name, ty, value } => {
                if self.scopes.last().unwrap().contains_key(name) {
                    return Err(Error::Sema(format!(
                        "variable `{}` already declared in this scope",
                        name
                    )));
                }
                let actual_ty = match (ty, value) {
                    (Some(t), _) => t.clone(),
                    (None, Some(v)) => self.infer(v)?,
                    (None, None) => unreachable!(
                        "parser should reject `xilit` without type or init"
                    ),
                };
                if let Some(v) = value {
                    // Empty array literal `[]` is only well-typed when the
                    // surrounding annotation provides the element type.
                    // Skip the check for that case — we know it's right.
                    let is_empty_array_lit_with_annotation = matches!(
                        (v, &actual_ty),
                        (Expr::ArrayLit(elems), Type::Array(_)) if elems.is_empty()
                    );
                    if !is_empty_array_lit_with_annotation {
                        let val_ty = self.infer(v)?;
                        if ty.is_some() && val_ty != actual_ty {
                            return Err(Error::Sema(format!(
                                "type mismatch: `{}` declared as {} but initializer is {}",
                                name,
                                type_name(&actual_ty),
                                type_name(&val_ty)
                            )));
                        }
                    }
                }
                self.declare(name, actual_ty);
            }
            Stmt::Assign { name, value } => {
                let target_ty = self.lookup(name).ok_or_else(|| {
                    Error::Sema(format!("assignment to undefined variable `{}`", name))
                })?;
                let val_ty = self.infer(value)?;
                if val_ty != target_ty {
                    return Err(Error::Sema(format!(
                        "type mismatch: `{}` is {} but value is {}",
                        name,
                        type_name(&target_ty),
                        type_name(&val_ty)
                    )));
                }
            }
            Stmt::IndexAssign { name, index, value } => {
                let arr_ty = self.lookup(name).ok_or_else(|| {
                    Error::Sema(format!("assignment to undefined variable `{}`", name))
                })?;
                let elem_ty = match &arr_ty {
                    Type::Array(inner) => (**inner).clone(),
                    other => {
                        return Err(Error::Sema(format!(
                            "`{}` is {}, can't index-assign",
                            name,
                            type_name(other)
                        )));
                    }
                };
                let idx_ty = self.infer(index)?;
                if idx_ty != Type::Terah {
                    return Err(Error::Sema(format!(
                        "array index must be terah, got {}",
                        type_name(&idx_ty)
                    )));
                }
                let val_ty = self.infer(value)?;
                if val_ty != elem_ty {
                    return Err(Error::Sema(format!(
                        "element type mismatch: array is [{}] but value is {}",
                        type_name(&elem_ty),
                        type_name(&val_ty)
                    )));
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                let ct = self.infer(cond)?;
                if ct != Type::Bool {
                    return Err(Error::Sema(format!(
                        "`nagah sanna` condition must be bool, got {}",
                        type_name(&ct)
                    )));
                }
                self.check_block(then_block)?;
                if let Some(eb) = else_block {
                    self.check_block(eb)?;
                }
            }
            Stmt::While { cond, body } => {
                let ct = self.infer(cond)?;
                if ct != Type::Bool {
                    return Err(Error::Sema(format!(
                        "`cqachunna` condition must be bool, got {}",
                        type_name(&ct)
                    )));
                }
                self.loop_depth += 1;
                let r = self.check_block(body);
                self.loop_depth -= 1;
                r?;
            }
            Stmt::ForEach { var, iter, body } => {
                let elem_ty = match iter {
                    IterSource::Range { start, end } => {
                        let st = self.infer(start)?;
                        if st != Type::Terah {
                            return Err(Error::Sema(format!(
                                "range start must be terah, got {}",
                                type_name(&st)
                            )));
                        }
                        let et = self.infer(end)?;
                        if et != Type::Terah {
                            return Err(Error::Sema(format!(
                                "range end must be terah, got {}",
                                type_name(&et)
                            )));
                        }
                        Type::Terah
                    }
                    IterSource::Array(arr_expr) => {
                        let arr_ty = self.infer(arr_expr)?;
                        match arr_ty {
                            Type::Array(inner) => *inner,
                            other => {
                                return Err(Error::Sema(format!(
                                    "`yallalc ... chu` needs an array, got {}",
                                    type_name(&other)
                                )));
                            }
                        }
                    }
                };
                self.loop_depth += 1;
                self.push_scope();
                self.declare(var, elem_ty);
                let r: Result<()> = (|| {
                    for s in &body.stmts {
                        self.check_stmt(s)?;
                    }
                    Ok(())
                })();
                self.pop_scope();
                self.loop_depth -= 1;
                r?;
            }
            Stmt::Break => {
                if self.loop_depth == 0 {
                    return Err(Error::Sema(
                        "`sac` outside of `cqachunna` loop".into(),
                    ));
                }
            }
            Stmt::Continue => {
                if self.loop_depth == 0 {
                    return Err(Error::Sema(
                        "`khida` outside of `cqachunna` loop".into(),
                    ));
                }
            }
            Stmt::Return(e) => {
                // Clone the expected return type out of `self` so we can
                // call `self.infer(...)` mutably below without holding an
                // immutable borrow across the call.
                let expected = self.current_return.clone();
                match (expected, e) {
                    (Some(rt), Some(expr)) => {
                        let et = self.infer(expr)?;
                        if et != rt {
                            return Err(Error::Sema(format!(
                                "return type mismatch: function expects {} but got {}",
                                type_name(&rt),
                                type_name(&et)
                            )));
                        }
                    }
                    (Some(rt), None) => {
                        return Err(Error::Sema(format!(
                            "function returns {} but `yuxadalo` has no value",
                            type_name(&rt)
                        )));
                    }
                    (None, Some(_)) => {
                        return Err(Error::Sema(
                            "void function can't return a value".into(),
                        ));
                    }
                    (None, None) => {}
                }
            }
            Stmt::Print(e) => {
                let ty = self.infer(e)?;
                if matches!(ty, Type::Array(_)) {
                    return Err(Error::Sema(
                        "can't print arrays directly yet — iterate \
                         with `yallalc` and print each element"
                            .into(),
                    ));
                }
            }
            Stmt::ExprStmt(e) => {
                // Allow void calls as statements (their value is discarded);
                // for everything else just type-check and toss the type.
                if let Expr::Call { callee, .. } = e {
                    if let Some(sig) = self.functions.get(callee).cloned() {
                        if sig.return_type.is_none() {
                            self.check_call(callee, e)?;
                            return Ok(());
                        }
                    }
                }
                self.infer(e)?;
            }
            Stmt::Push { name, value } => {
                let arr_ty = self.lookup(name).ok_or_else(|| {
                    Error::Sema(format!("push on undefined variable `{}`", name))
                })?;
                let elem_ty = match &arr_ty {
                    Type::Array(inner) => (**inner).clone(),
                    other => {
                        return Err(Error::Sema(format!(
                            "`push` needs an array, but `{}` is {}",
                            name,
                            type_name(other)
                        )));
                    }
                };
                if self.current_params.contains(name) {
                    return Err(Error::Sema(format!(
                        "cannot push to `{}`: it's a function parameter, \
                         and mott doesn't have references yet — push/pop \
                         only work on locals",
                        name
                    )));
                }
                let val_ty = self.infer(value)?;
                if val_ty != elem_ty {
                    return Err(Error::Sema(format!(
                        "push type mismatch: `{}` holds {}, got {}",
                        name,
                        type_name(&elem_ty),
                        type_name(&val_ty)
                    )));
                }
            }
        }
        Ok(())
    }

    /// Type-check a function call and return its return type. Used in
    /// both expression contexts (`emit_expr`) and statement contexts
    /// (`ExprStmt` for void calls). Caller decides what to do with the
    /// return type (or its absence).
    fn check_call(&mut self, callee: &str, e: &Expr) -> Result<Option<Type>> {
        let Expr::Call { args, .. } = e else {
            unreachable!("check_call requires Expr::Call");
        };
        let sig = self
            .functions
            .get(callee)
            .cloned()
            .ok_or_else(|| Error::Sema(format!("call to undefined function `{}`", callee)))?;
        if sig.params.len() != args.len() {
            return Err(Error::Sema(format!(
                "function `{}` expects {} args, got {}",
                callee,
                sig.params.len(),
                args.len()
            )));
        }
        for (i, (arg, expected)) in args.iter().zip(sig.params.iter()).enumerate() {
            let at = self.infer(arg)?;
            if at != *expected {
                return Err(Error::Sema(format!(
                    "argument {} of `{}`: expected {}, got {}",
                    i + 1,
                    callee,
                    type_name(expected),
                    type_name(&at)
                )));
            }
        }
        Ok(sig.return_type)
    }

    /// Infer the type of an expression while validating it. This is the
    /// only type-walk: codegen will run its own much-simpler version that
    /// trusts the AST is well-formed.
    fn infer(&mut self, e: &Expr) -> Result<Type> {
        match e {
            Expr::Integer(_) => Ok(Type::Terah),
            Expr::Float(_) => Ok(Type::Daqosh),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::String(parts) => {
                // Validate every interpolated expression and ensure its
                // type is something we know how to stringify.
                for p in parts {
                    if let StringPart::Interpolation(expr) = p {
                        let t = self.infer(expr)?;
                        if matches!(t, Type::Array(_)) {
                            return Err(Error::Sema(
                                "cannot interpolate an array into a string — \
                                 iterate and interpolate each element"
                                    .into(),
                            ));
                        }
                    }
                }
                Ok(Type::Deshnash)
            }
            Expr::Ident(name) => self
                .lookup(name)
                .ok_or_else(|| Error::Sema(format!("use of undefined variable `{}`", name))),
            Expr::Binary { op, left, right } => {
                let lt = self.infer(left)?;
                let rt = self.infer(right)?;
                match op {
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                        if lt != rt {
                            return Err(Error::Sema(format!(
                                "arithmetic type mismatch: {} vs {}",
                                type_name(&lt),
                                type_name(&rt)
                            )));
                        }
                        if !matches!(lt, Type::Terah | Type::Daqosh) {
                            return Err(Error::Sema(format!(
                                "arithmetic on non-numeric type {}",
                                type_name(&lt)
                            )));
                        }
                        if matches!(op, BinOp::Mod) && lt != Type::Terah {
                            return Err(Error::Sema(
                                "`%` is only defined for terah".into(),
                            ));
                        }
                        Ok(lt)
                    }
                    BinOp::Eq | BinOp::NotEq => {
                        if lt != rt {
                            return Err(Error::Sema(format!(
                                "comparison type mismatch: {} vs {}",
                                type_name(&lt),
                                type_name(&rt)
                            )));
                        }
                        Ok(Type::Bool)
                    }
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        if lt != rt {
                            return Err(Error::Sema(format!(
                                "comparison type mismatch: {} vs {}",
                                type_name(&lt),
                                type_name(&rt)
                            )));
                        }
                        if !matches!(lt, Type::Terah | Type::Daqosh) {
                            return Err(Error::Sema(format!(
                                "ordering comparison needs numeric operands, got {}",
                                type_name(&lt)
                            )));
                        }
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::Unary { op, expr } => {
                let t = self.infer(expr)?;
                match op {
                    UnOp::Neg => {
                        if !matches!(t, Type::Terah | Type::Daqosh) {
                            return Err(Error::Sema(format!(
                                "unary `-` requires numeric, got {}",
                                type_name(&t)
                            )));
                        }
                        Ok(t)
                    }
                    UnOp::Not => {
                        if t != Type::Bool {
                            return Err(Error::Sema(format!(
                                "unary `!` requires bool, got {}",
                                type_name(&t)
                            )));
                        }
                        Ok(Type::Bool)
                    }
                }
            }
            Expr::LogicAnd(ops) | Expr::LogicOr(ops) => {
                let kind = if matches!(e, Expr::LogicAnd(_)) {
                    "AND"
                } else {
                    "OR"
                };
                for op in ops {
                    let t = self.infer(op)?;
                    if t != Type::Bool {
                        return Err(Error::Sema(format!(
                            "{} operand must be bool, got {}",
                            kind,
                            type_name(&t)
                        )));
                    }
                }
                Ok(Type::Bool)
            }
            Expr::Call { callee, .. } => {
                let ret = self.check_call(callee, e)?;
                ret.ok_or_else(|| {
                    Error::Sema(format!(
                        "function `{}` returns no value but its result is used",
                        callee
                    ))
                })
            }
            Expr::Input => Ok(Type::Deshnash),
            Expr::ArrayLit(elems) => {
                if elems.is_empty() {
                    return Err(Error::Sema(
                        "empty array literal `[]` needs a type annotation; \
                         write `xilit nums: [terah] = []` or use `[1, 2, 3]`"
                            .into(),
                    ));
                }
                let first_ty = self.infer(&elems[0])?;
                if matches!(first_ty, Type::Array(_)) {
                    return Err(Error::Sema(
                        "nested arrays aren't supported yet".into(),
                    ));
                }
                for (i, el) in elems.iter().enumerate().skip(1) {
                    let t = self.infer(el)?;
                    if t != first_ty {
                        return Err(Error::Sema(format!(
                            "array literal element {}: expected {}, got {}",
                            i,
                            type_name(&first_ty),
                            type_name(&t)
                        )));
                    }
                }
                Ok(Type::Array(Box::new(first_ty)))
            }
            Expr::Index { target, index } => {
                let tgt_ty = self.infer(target)?;
                let elem_ty = match tgt_ty {
                    Type::Array(inner) => *inner,
                    other => {
                        return Err(Error::Sema(format!(
                            "cannot index into non-array type {}",
                            type_name(&other)
                        )));
                    }
                };
                let idx_ty = self.infer(index)?;
                if idx_ty != Type::Terah {
                    return Err(Error::Sema(format!(
                        "array index must be terah, got {}",
                        type_name(&idx_ty)
                    )));
                }
                Ok(elem_ty)
            }
            Expr::Baram(inner) => {
                let t = self.infer(inner)?;
                if !matches!(t, Type::Array(_) | Type::Deshnash) {
                    return Err(Error::Sema(format!(
                        "`baram` needs an array or string, got {}",
                        type_name(&t)
                    )));
                }
                Ok(Type::Terah)
            }
            Expr::ParseTerah(inner) => {
                let t = self.infer(inner)?;
                if t != Type::Deshnash {
                    return Err(Error::Sema(format!(
                        "`parse_terah` needs a deshnash, got {}",
                        type_name(&t)
                    )));
                }
                Ok(Type::Terah)
            }
            Expr::ParseDaqosh(inner) => {
                let t = self.infer(inner)?;
                if t != Type::Deshnash {
                    return Err(Error::Sema(format!(
                        "`parse_daqosh` needs a deshnash, got {}",
                        type_name(&t)
                    )));
                }
                Ok(Type::Daqosh)
            }
            Expr::ToTerah(inner) => {
                let t = self.infer(inner)?;
                if !matches!(t, Type::Terah | Type::Daqosh) {
                    return Err(Error::Sema(format!(
                        "`to_terah` needs a numeric type (terah or daqosh), got {}",
                        type_name(&t)
                    )));
                }
                Ok(Type::Terah)
            }
            Expr::ToDaqosh(inner) => {
                let t = self.infer(inner)?;
                if !matches!(t, Type::Terah | Type::Daqosh) {
                    return Err(Error::Sema(format!(
                        "`to_daqosh` needs a numeric type (terah or daqosh), got {}",
                        type_name(&t)
                    )));
                }
                Ok(Type::Daqosh)
            }
            Expr::Pop(name) => {
                let arr_ty = self.lookup(name).ok_or_else(|| {
                    Error::Sema(format!("pop on undefined variable `{}`", name))
                })?;
                let elem_ty = match arr_ty {
                    Type::Array(inner) => *inner,
                    other => {
                        return Err(Error::Sema(format!(
                            "`pop` needs an array, but `{}` is {}",
                            name,
                            type_name(&other)
                        )));
                    }
                };
                if self.current_params.contains(name) {
                    return Err(Error::Sema(format!(
                        "cannot pop from `{}`: it's a function parameter \
                         (see push error message for why)",
                        name
                    )));
                }
                Ok(elem_ty)
            }
        }
    }

    // --- scope helpers ---
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }
    fn declare(&mut self, name: &str, ty: Type) {
        self.scopes
            .last_mut()
            .expect("no active scope")
            .insert(name.to_string(), ty);
    }
    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }
}

/// Mott-language type name for diagnostics.
fn type_name(t: &Type) -> String {
    match t {
        Type::Terah => "terah".into(),
        Type::Daqosh => "daqosh".into(),
        Type::Bool => "bool".into(),
        Type::Deshnash => "deshnash".into(),
        Type::Array(inner) => format!("[{}]", type_name(inner)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn check_src(src: &str) -> Result<TypeInfo> {
        let tokens = Lexer::new(src).tokenize()?;
        let program = Parser::new(tokens).parse()?;
        check(&program)
    }

    #[test]
    fn well_typed_program_passes() {
        check_src(
            r#"
fnc kort() {
    xilit x: terah = 5
    yazde("x = {x}")
}
"#,
        )
        .expect("should pass sema");
    }

    #[test]
    fn type_mismatch_in_let_errors() {
        let err = check_src("fnc kort() { xilit x: terah = baqderg; }").unwrap_err();
        assert!(format!("{}", err).contains("type mismatch"));
    }

    #[test]
    fn undefined_variable_errors() {
        let err = check_src("fnc kort() { yazde(nope); }").unwrap_err();
        assert!(format!("{}", err).contains("undefined variable"));
    }

    #[test]
    fn sac_outside_loop_errors() {
        let err = check_src("fnc kort() { sac; }").unwrap_err();
        assert!(format!("{}", err).contains("outside of"));
    }

    #[test]
    fn duplicate_function_errors() {
        let err = check_src(
            r#"
fnc helper() {}
fnc helper() {}
fnc kort() {}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("duplicate function"));
    }

    #[test]
    fn kort_with_params_rejected() {
        let err = check_src("fnc kort(x: terah) { yazde(x); }").unwrap_err();
        assert!(format!("{}", err).contains("kort"));
    }

    #[test]
    fn kort_with_return_type_rejected() {
        let err = check_src(
            r#"
fnc kort() -> terah {
    yuxadalo 0
}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("kort"));
    }

    #[test]
    fn arithmetic_on_strings_rejected() {
        let err = check_src(
            r#"fnc kort() {
    xilit x = "a" + "b"
}"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("non-numeric"));
    }

    #[test]
    fn return_type_must_match_function_signature() {
        let err = check_src(
            r#"
fnc add(x: terah, y: terah) -> terah {
    yuxadalo baqderg
}
fnc kort() {}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("return type mismatch"));
    }

    #[test]
    fn void_function_returning_value_rejected() {
        let err = check_src(
            r#"
fnc helper() {
    yuxadalo 5
}
fnc kort() {}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("void function"));
    }

    #[test]
    fn non_void_function_returning_nothing_rejected() {
        let err = check_src(
            r#"
fnc helper() -> terah {
    yuxadalo
}
fnc kort() {}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("no value"));
    }

    #[test]
    fn push_on_parameter_rejected() {
        let err = check_src(
            r#"
fnc helper(arr: [terah]) {
    push(arr, 1)
}
fnc kort() {}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("function parameter"));
    }

    #[test]
    fn push_type_mismatch_rejected() {
        let err = check_src(
            r#"
fnc kort() {
    xilit nums: [terah] = [1]
    push(nums, baqderg)
}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("push type mismatch"));
    }

    #[test]
    fn interpolation_with_array_rejected() {
        let err = check_src(
            r#"
fnc kort() {
    xilit nums: [terah] = [1, 2]
    yazde("{nums}")
}
"#,
        )
        .unwrap_err();
        assert!(format!("{}", err).contains("interpolate an array"));
    }
}
