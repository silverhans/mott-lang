//! C backend: walks the Mott AST and emits portable C11.
//!
//! After sema split this is a near-mechanical lowering. The AST is
//! assumed type-correct (sema runs first); any type error here is a
//! compiler bug, not a user error. We still keep enough type info around
//! to pick the right monomorphic runtime helper (`mott_yazde_terah` vs
//! `mott_yazde_deshnash`, etc.) and the right C type for declarations.
//!
//! Notes on the runtime contract (see `runtime/mott_rt.h`):
//! - `terah` maps to `int64_t`, `daqosh` to `double`, `bool` to `bool`,
//!   `deshnash` to `mott_str` (struct with `data` + `len`).
//! - String interpolation lowers to `mott_str_build(parts, n)` over a
//!   compound-literal array of `mott_str`s.
//! - `yazde` dispatches per type to a runtime helper picked at codegen.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::ast::{
    BinOp, Block, Expr, Function, Item, IterSource, Program, Stmt, StringPart, StructDef, Type,
    UnOp,
};
use crate::codegen::Backend;
use crate::error::Result;

pub struct CBackend;

impl Backend for CBackend {
    fn name(&self) -> &'static str {
        "c"
    }

    fn emit(&self, program: &Program) -> Result<String> {
        let mut e = Emitter::new(program);
        e.emit_program(program);
        Ok(e.out)
    }
}

/// Codegen only needs the return type to dispatch values back from
/// calls; arity and parameter types were validated by sema. Distinct
/// from `sema::FuncSig` to keep the codegen surface minimal.
#[derive(Debug, Clone)]
struct FuncSig {
    return_type: Option<Type>,
}

struct Emitter {
    out: String,
    indent: usize,
    /// Variable types per scope. Sema's already validated everything;
    /// we keep a table here purely for emit-time needs (picking C type
    /// names, resolving an ident's type to dispatch the right runtime
    /// helper).
    scopes: Vec<HashMap<String, Type>>,
    functions: HashMap<String, FuncSig>,
    /// User-defined structs, keyed by name. Used to look up field order
    /// at struct-literal emit time so we can emit fields in declaration
    /// order regardless of source-literal order.
    structs: HashMap<String, StructDef>,
}

impl Emitter {
    fn new(program: &Program) -> Self {
        let mut functions = HashMap::new();
        let mut structs = HashMap::new();
        for item in &program.items {
            match item {
                Item::Function(f) => {
                    let key = match &f.module {
                        Some(m) => format!("{}.{}", m, f.name),
                        None => f.name.clone(),
                    };
                    functions.insert(
                        key,
                        FuncSig {
                            return_type: f.return_type.clone(),
                        },
                    );
                }
                Item::Struct(s) => {
                    structs.insert(s.name.clone(), s.clone());
                }
                // Imports were resolved by the loader; if any survive
                // here it's because of a unit test that bypassed loading
                // — skip them.
                Item::Import { .. } => {}
            }
        }
        Self {
            out: String::new(),
            indent: 0,
            scopes: Vec::new(),
            functions,
            structs,
        }
    }

    fn emit_program(&mut self, program: &Program) {
        self.write_prelude();

        // Emit struct typedefs in topological order: each struct must be
        // fully defined before any other struct that contains it by value.
        // Sema already verified there are no cycles.
        let struct_order = topo_sort_structs(&self.structs);
        for s in &struct_order {
            self.emit_struct_typedef(s);
        }
        if !struct_order.is_empty() {
            self.writeln("");
        }

        // Per-struct array machinery: any user struct that's used as
        // an array element type needs its own `mott_arr_<Name>` typedef
        // and ops. Collect those names by scanning the program for
        // `[StructName]` types.
        let struct_array_uses = collect_struct_array_uses(program);
        for name in &struct_array_uses {
            self.emit_struct_array_machinery(name);
        }
        if !struct_array_uses.is_empty() {
            self.writeln("");
        }

        // Forward-declare every non-entry function so call-before-define
        // works. Extern functions (no body) get only the forward decl;
        // they're implemented in the runtime / module C files.
        let mut has_decls = false;
        for item in &program.items {
            if let Item::Function(f) = item {
                if f.name == "kort" {
                    continue;
                }
                self.emit_function_signature(f);
                self.writeln(";");
                has_decls = true;
            }
        }
        if has_decls {
            self.writeln("");
        }

        for item in &program.items {
            if let Item::Function(f) = item {
                if f.body.is_some() {
                    self.emit_function(f);
                    self.writeln("");
                }
            }
        }
    }

    fn emit_struct_typedef(&mut self, s: &StructDef) {
        self.writeln(&format!("typedef struct {} {{", s.name));
        for field in &s.fields {
            self.writeln(&format!("    {} {};", type_to_c(&field.ty), field.name));
        }
        // Empty structs need a placeholder field (C forbids zero-field
        // structs; gcc accepts them as extension but clang -std=c11 warns).
        if s.fields.is_empty() {
            self.writeln("    char __mott_empty;");
        }
        self.writeln(&format!("}} {};", s.name));
    }

    /// Emit per-struct array typedef + new/push/pop helpers as `static
    /// inline` so they can sit in any compilation unit. We only emit
    /// these for structs actually used as array elements.
    fn emit_struct_array_machinery(&mut self, name: &str) {
        // Type
        self.writeln(&format!(
            "typedef struct {{ {0} *data; size_t len; size_t cap; }} mott_arr_{0};",
            name
        ));
        // _new
        self.writeln(&format!(
            "static inline mott_arr_{0} mott_arr_{0}_new(size_t n, const {0} *src) {{",
            name
        ));
        self.writeln("    size_t cap = n > 4 ? n : 4;");
        self.writeln(&format!(
            "    {0} *data = ({0} *)malloc(cap * sizeof({0}));",
            name
        ));
        self.writeln("    if (!data) { fputs(\"mott runtime: out of memory\\n\", stderr); abort(); }");
        self.writeln(&format!(
            "    if (n > 0) memcpy(data, src, n * sizeof({0}));",
            name
        ));
        self.writeln(&format!(
            "    return (mott_arr_{0}){{ .data = data, .len = n, .cap = cap }};",
            name
        ));
        self.writeln("}");
        // _push
        self.writeln(&format!(
            "static inline void mott_arr_{0}_push(mott_arr_{0} *a, {0} x) {{",
            name
        ));
        self.writeln("    if (a->len == a->cap) {");
        self.writeln("        size_t new_cap = a->cap * 2;");
        self.writeln(&format!(
            "        a->data = ({0} *)realloc(a->data, new_cap * sizeof({0}));",
            name
        ));
        self.writeln("        if (!a->data) { fputs(\"mott runtime: out of memory\\n\", stderr); abort(); }");
        self.writeln("        a->cap = new_cap;");
        self.writeln("    }");
        self.writeln("    a->data[a->len++] = x;");
        self.writeln("}");
        // _pop
        self.writeln(&format!(
            "static inline {0} mott_arr_{0}_pop(mott_arr_{0} *a) {{",
            name
        ));
        self.writeln("    if (a->len == 0) { fputs(\"mott runtime: pop on empty array\\n\", stderr); abort(); }");
        self.writeln("    return a->data[--a->len];");
        self.writeln("}");
    }

    fn write_prelude(&mut self) {
        self.writeln("/* Generated by mott compiler v0.3 — do not edit. */");
        self.writeln("#include <stdbool.h>");
        self.writeln("#include <stdint.h>");
        // The runtime header pulls in stddef for size_t. We additionally
        // need stdlib (malloc/realloc/abort) and string (memcpy) and
        // stdio (fputs) because the per-struct array machinery emits
        // inline functions that use them — even when no user struct
        // requires them, the cost is just a few include lines.
        self.writeln("#include <stdio.h>");
        self.writeln("#include <stdlib.h>");
        self.writeln("#include <string.h>");
        self.writeln("#include \"mott_rt.h\"");
        self.writeln("");
    }

    fn emit_function_signature(&mut self, f: &Function) {
        let ret = match &f.return_type {
            Some(t) => type_to_c(t),
            None => "void".into(),
        };
        let name = c_func_name(&f.module, &f.name);
        self.write(&format!("{} {}(", ret, name));
        if f.params.is_empty() {
            self.write("void");
        } else {
            for (i, p) in f.params.iter().enumerate() {
                if i > 0 {
                    self.write(", ");
                }
                self.write(&format!("{} {}", type_to_c(&p.ty), p.name));
            }
        }
        self.write(")");
    }

    fn emit_function(&mut self, f: &Function) {
        // Extern functions: nothing to emit here. Their forward declaration
        // lives in the prelude (see `emit_program`).
        let body = match &f.body {
            Some(b) => b,
            None => return,
        };
        if f.name == "kort" {
            // Sema has already verified `kort` takes no params and has no
            // return type — we just emit `int main(void)` here.
            self.write("int main(void)");
        } else {
            self.emit_function_signature(f);
        }
        self.writeln(" {");
        self.indent += 1;
        self.push_scope();
        for p in &f.params {
            self.declare(&p.name, p.ty.clone());
        }
        for s in &body.stmts {
            self.emit_stmt(s);
        }
        if f.name == "kort" {
            self.write_indent();
            self.writeln("return 0;");
        }
        self.pop_scope();
        self.indent -= 1;
        self.writeln("}");
    }

    /// Emit a `yallalc var chu iter { body }` loop. For arrays we iterate
    /// by index and bind `var` to the element; for ranges we emit a plain
    /// counting loop.
    fn emit_for_each(&mut self, var: &str, iter: &IterSource, body: &Block) {
        match iter {
            IterSource::Range { start, end } => {
                self.write_indent();
                self.write(&format!("for (int64_t {} = ", var));
                self.emit_expr(start);
                self.write(&format!("; {} < ", var));
                self.emit_expr(end);
                self.write(&format!("; {}++)", var));
                self.push_scope();
                self.declare(var, Type::Terah);
                self.emit_block_body(body);
                self.pop_scope();
                self.writeln("");
            }
            IterSource::Array(arr_expr) => {
                // { TYPE __mott_arr = <arr>; for (size_t __mott_i = 0; ...) {
                //     ELEM var = __mott_arr.data[__mott_i]; <body>
                // } }
                let arr_ty = self.type_of(arr_expr);
                let elem_ty = match arr_ty {
                    Type::Array(inner) => *inner,
                    _ => unreachable!("sema ensures yallalc source is array"),
                };
                self.write_indent();
                self.writeln("{");
                self.indent += 1;
                self.write_indent();
                self.write(&format!(
                    "{} __mott_arr = ",
                    type_to_c(&Type::Array(Box::new(elem_ty.clone())))
                ));
                self.emit_expr(arr_expr);
                self.writeln(";");
                self.write_indent();
                self.write("for (size_t __mott_i = 0; __mott_i < __mott_arr.len; __mott_i++)");
                self.writeln(" {");
                self.indent += 1;
                self.write_indent();
                self.writeln(&format!(
                    "{} {} = __mott_arr.data[__mott_i];",
                    type_to_c(&elem_ty),
                    var
                ));
                self.push_scope();
                self.declare(var, elem_ty);
                for s in &body.stmts {
                    self.emit_stmt(s);
                }
                self.pop_scope();
                self.indent -= 1;
                self.write_indent();
                self.writeln("}");
                self.indent -= 1;
                self.write_indent();
                self.writeln("}");
            }
        }
    }

    /// Emit just the statements of a block, without opening/closing braces
    /// or pushing a scope. Used by for-each (range case) where the caller
    /// manages bracing/scoping itself.
    fn emit_block_body(&mut self, b: &Block) {
        self.writeln(" {");
        self.indent += 1;
        self.push_scope();
        for s in &b.stmts {
            self.emit_stmt(s);
        }
        self.pop_scope();
        self.indent -= 1;
        self.write_indent();
        self.write("}");
    }

    /// Emit `callee(arg1, ...)`. Sema validated arity + types already.
    /// Module-qualified calls get the `mott_<module>_<name>` mangling so
    /// they don't collide with libc symbols.
    fn emit_call(&mut self, module: &Option<String>, callee: &str, args: &[Expr]) {
        let name = c_func_name(module, callee);
        self.write(&name);
        self.write("(");
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            self.emit_expr(arg);
        }
        self.write(")");
    }

    /// Emit `if (cond) {...} [else ...]`, collapsing parser-desugared
    /// `vusht nagah sanna` chains back into idiomatic C `else if`.
    fn emit_if_chain(&mut self, cond: &Expr, then_block: &Block, else_block: Option<&Block>) {
        self.write("if (");
        self.emit_expr(cond);
        self.write(")");
        self.emit_block_inline(then_block);
        if let Some(eb) = else_block {
            if let Some((c2, t2, e2)) = as_chained_if(eb) {
                self.write(" else ");
                self.emit_if_chain(c2, t2, e2);
            } else {
                self.write(" else");
                self.emit_block_inline(eb);
            }
        }
    }

    fn emit_block_inline(&mut self, b: &Block) {
        self.writeln(" {");
        self.indent += 1;
        self.push_scope();
        for s in &b.stmts {
            self.emit_stmt(s);
        }
        self.pop_scope();
        self.indent -= 1;
        self.write_indent();
        self.write("}");
    }

    fn emit_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let { name, ty, value } => {
                self.write_indent();
                let actual_ty = match (ty, value) {
                    (Some(t), _) => t.clone(),
                    (None, Some(v)) => self.type_of(v),
                    (None, None) => unreachable!("sema rejects xilit without type or init"),
                };
                self.write(&format!("{} {} = ", type_to_c(&actual_ty), name));
                match value {
                    None => self.write(&zero_value(&actual_ty)),
                    Some(Expr::ArrayLit(elems))
                        if elems.is_empty() && matches!(actual_ty, Type::Array(_)) =>
                    {
                        let Type::Array(elem_ty) = &actual_ty else {
                            unreachable!()
                        };
                        let ctor = array_ctor_name(elem_ty);
                        self.write(&format!("{}(0, NULL)", ctor));
                    }
                    Some(v) => {
                        self.emit_expr(v);
                    }
                }
                self.writeln(";");
                self.declare(name, actual_ty);
            }
            Stmt::Assign { name, value } => {
                self.write_indent();
                self.write(&format!("{} = ", name));
                self.emit_expr(value);
                self.writeln(";");
            }
            Stmt::IndexAssign { name, index, value } => {
                self.write_indent();
                self.write(&format!("{}.data[", name));
                self.emit_expr(index);
                self.write("] = ");
                self.emit_expr(value);
                self.writeln(";");
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                self.write_indent();
                self.emit_if_chain(cond, then_block, else_block.as_ref());
                self.writeln("");
            }
            Stmt::While { cond, body } => {
                self.write_indent();
                self.write("while (");
                self.emit_expr(cond);
                self.write(")");
                self.emit_block_inline(body);
                self.writeln("");
            }
            Stmt::ForEach { var, iter, body } => {
                self.emit_for_each(var, iter, body);
            }
            Stmt::Break => {
                self.write_indent();
                self.writeln("break;");
            }
            Stmt::Continue => {
                self.write_indent();
                self.writeln("continue;");
            }
            Stmt::Return(e) => {
                self.write_indent();
                match e {
                    Some(expr) => {
                        self.write("return ");
                        self.emit_expr(expr);
                        self.writeln(";");
                    }
                    None => self.writeln("return;"),
                }
            }
            Stmt::Print(e) => {
                let ty = self.type_of(e);
                let helper = match ty {
                    Type::Terah => "mott_yazde_terah",
                    Type::Daqosh => "mott_yazde_daqosh",
                    Type::Bool => "mott_yazde_bool",
                    Type::Deshnash => "mott_yazde_deshnash",
                    Type::Array(_) | Type::Struct(_) => {
                        unreachable!("sema rejects yazde of array/struct")
                    }
                };
                self.write_indent();
                self.write(&format!("{}(", helper));
                self.emit_expr(e);
                self.writeln(");");
            }
            Stmt::ExprStmt(e) => {
                self.write_indent();
                // Void calls are valid as bare statements but the value
                // path of emit_expr would try to use the result. Detect
                // and route around.
                if let Expr::Call {
                    module,
                    callee,
                    args,
                } = e
                {
                    let key = match module {
                        Some(m) => format!("{}.{}", m, callee),
                        None => callee.clone(),
                    };
                    if let Some(sig) = self.functions.get(&key) {
                        if sig.return_type.is_none() {
                            self.emit_call(module, callee, args);
                            self.writeln(";");
                            return;
                        }
                    }
                }
                self.emit_expr(e);
                self.writeln(";");
            }
            Stmt::Push { name, value } => {
                let arr_ty = self.lookup(name).expect("sema ensures var defined");
                let elem_ty = match arr_ty {
                    Type::Array(inner) => *inner,
                    _ => unreachable!("sema ensures push target is array"),
                };
                let push_fn = array_push_name(&elem_ty);
                self.write_indent();
                self.write(&format!("{}(&{}, ", push_fn, name));
                self.emit_expr(value);
                self.writeln(");");
            }
            Stmt::FieldAssign {
                target,
                field,
                value,
            } => {
                self.write_indent();
                self.write(&format!("{}.{} = ", target, field));
                self.emit_expr(value);
                self.writeln(";");
            }
        }
    }

    /// Emit an expression. The AST is type-correct (sema), so we can
    /// shape the output without checks. Wraps in parens defensively to
    /// avoid precedence surprises — the C compiler cleans them up.
    fn emit_expr(&mut self, e: &Expr) {
        match e {
            Expr::Integer(n) => {
                if *n == i64::MIN {
                    self.write("((int64_t)(-9223372036854775807LL - 1))");
                } else {
                    self.write(&format!("((int64_t){}LL)", n));
                }
            }
            Expr::Float(f) => {
                let s = format!("{:?}", f);
                let s = if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("nan") {
                    s
                } else {
                    format!("{}.0", s)
                };
                self.write(&s);
            }
            Expr::Bool(b) => {
                self.write(if *b { "true" } else { "false" });
            }
            Expr::Ident(name) => {
                self.write(name);
            }
            Expr::String(parts) => {
                self.emit_string_expr(parts);
            }
            Expr::Binary { op, left, right } => {
                // Special case: deshnash equality lowers to a runtime
                // call (struct compare via `==` would just compare the
                // data pointer).
                if matches!(op, BinOp::Eq | BinOp::NotEq)
                    && self.type_of(left) == Type::Deshnash
                {
                    let negate = matches!(op, BinOp::NotEq);
                    if negate {
                        self.write("(!mott_str_eq(");
                    } else {
                        self.write("mott_str_eq(");
                    }
                    self.emit_expr(left);
                    self.write(", ");
                    self.emit_expr(right);
                    self.write(if negate { "))" } else { ")" });
                    return;
                }
                self.write("(");
                self.emit_expr(left);
                self.write(bin_op_str(*op));
                self.emit_expr(right);
                self.write(")");
            }
            Expr::Unary { op, expr } => {
                self.write("(");
                self.write(match op {
                    UnOp::Neg => "-",
                    UnOp::Not => "!",
                });
                self.emit_expr(expr);
                self.write(")");
            }
            Expr::LogicAnd(ops) => {
                self.write("(");
                for (i, op) in ops.iter().enumerate() {
                    if i > 0 {
                        self.write(" && ");
                    }
                    self.emit_expr(op);
                }
                self.write(")");
            }
            Expr::LogicOr(ops) => {
                self.write("(");
                for (i, op) in ops.iter().enumerate() {
                    if i > 0 {
                        self.write(" || ");
                    }
                    self.emit_expr(op);
                }
                self.write(")");
            }
            Expr::Call {
                module,
                callee,
                args,
            } => {
                self.emit_call(module, callee, args);
            }
            Expr::Input => {
                self.write("mott_input()");
            }
            Expr::ArrayLit(elems) => {
                // Sema rejects empty literals here (Stmt::Let handles the
                // typed-context empty case before reaching emit_expr).
                let first_ty = self.type_of(&elems[0]);
                let elem_c = type_to_c(&first_ty);
                let ctor = array_ctor_name(&first_ty);
                self.write(&format!("{}({}, (", ctor, elems.len()));
                self.write(&format!("{}[]){{ ", elem_c));
                for (i, el) in elems.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.emit_expr(el);
                }
                self.write(" })");
            }
            Expr::Index { target, index } => {
                self.write("(");
                self.emit_expr(target);
                self.write(".data[");
                self.emit_expr(index);
                self.write("])");
            }
            Expr::Baram(inner) => {
                // Both mott_arr_* and mott_str have `.len`; cast to
                // int64_t since size_t isn't a Mott type.
                self.write("((int64_t)(");
                self.emit_expr(inner);
                self.write(".len))");
            }
            Expr::ParseTerah(inner) => {
                self.write("mott_parse_terah(");
                self.emit_expr(inner);
                self.write(")");
            }
            Expr::ParseDaqosh(inner) => {
                self.write("mott_parse_daqosh(");
                self.emit_expr(inner);
                self.write(")");
            }
            Expr::ToTerah(inner) => {
                self.write("((int64_t)(");
                self.emit_expr(inner);
                self.write("))");
            }
            Expr::ToDaqosh(inner) => {
                self.write("((double)(");
                self.emit_expr(inner);
                self.write("))");
            }
            Expr::Pop(name) => {
                let arr_ty = self.lookup(name).expect("sema ensures var defined");
                let elem_ty = match arr_ty {
                    Type::Array(inner) => *inner,
                    _ => unreachable!(),
                };
                let pop_fn = array_pop_name(&elem_ty);
                self.write(&format!("{}(&{})", pop_fn, name));
            }
            Expr::StructLit { name, fields } => {
                // Emit fields in declaration order, regardless of source
                // order. `((Point){.x = 3, .y = 5})` — designator init.
                let def = self
                    .structs
                    .get(name)
                    .expect("sema ensures struct exists")
                    .clone();
                self.write(&format!("(({}){{", name));
                let by_name: HashMap<&str, &Expr> = fields
                    .iter()
                    .map(|(n, e)| (n.as_str(), e))
                    .collect();
                for (i, field) in def.fields.iter().enumerate() {
                    if i > 0 {
                        self.write(", ");
                    }
                    self.write(&format!(".{} = ", field.name));
                    let expr = by_name
                        .get(field.name.as_str())
                        .expect("sema ensures all fields supplied");
                    self.emit_expr(expr);
                }
                if def.fields.is_empty() {
                    // Empty structs got a `__mott_empty` placeholder; init it.
                    self.write(".__mott_empty = 0");
                }
                self.write("})");
            }
            Expr::FieldAccess { target, field } => {
                self.write("(");
                self.emit_expr(target);
                self.write(&format!(".{})", field));
            }
        }
    }

    fn emit_string_expr(&mut self, parts: &[StringPart]) {
        // No interpolations → single literal.
        let has_interp = parts
            .iter()
            .any(|p| matches!(p, StringPart::Interpolation(_)));
        if !has_interp {
            let combined: String = parts
                .iter()
                .map(|p| match p {
                    StringPart::Literal(s) => s.as_str(),
                    _ => unreachable!(),
                })
                .collect();
            self.write(&format!("MOTT_STR_LIT({})", c_string_literal(&combined)));
            return;
        }

        // Interpolated: build an array of mott_str parts and hand to runtime.
        self.write("mott_str_build((mott_str[]){ ");
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                self.write(", ");
            }
            match part {
                StringPart::Literal(s) => {
                    self.write(&format!("MOTT_STR_LIT({})", c_string_literal(s)));
                }
                StringPart::Interpolation(expr) => {
                    let ty = self.type_of(expr);
                    match ty {
                        Type::Deshnash => {
                            self.emit_expr(expr);
                        }
                        Type::Terah => {
                            self.write("mott_str_from_terah(");
                            self.emit_expr(expr);
                            self.write(")");
                        }
                        Type::Daqosh => {
                            self.write("mott_str_from_daqosh(");
                            self.emit_expr(expr);
                            self.write(")");
                        }
                        Type::Bool => {
                            self.write("mott_str_from_bool(");
                            self.emit_expr(expr);
                            self.write(")");
                        }
                        Type::Array(_) | Type::Struct(_) => {
                            unreachable!("sema rejects array/struct interpolation")
                        }
                    }
                }
            }
        }
        self.write(&format!(" }}, {})", parts.len()));
    }

    /// Light type inference used to pick the right C type / runtime helper.
    /// Sema has already validated the AST, so any unexpected shape here is
    /// a compiler bug — we panic with `unreachable!` rather than threading
    /// a `Result` through every emit call.
    fn type_of(&self, e: &Expr) -> Type {
        match e {
            Expr::Integer(_) => Type::Terah,
            Expr::Float(_) => Type::Daqosh,
            Expr::Bool(_) => Type::Bool,
            Expr::String(_) => Type::Deshnash,
            Expr::Ident(name) => self
                .lookup(name)
                .unwrap_or_else(|| unreachable!("sema ensures `{}` is defined", name)),
            Expr::Binary { op, left, .. } => match op {
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    Type::Bool
                }
                _ => self.type_of(left),
            },
            Expr::Unary { op, expr } => match op {
                UnOp::Not => Type::Bool,
                UnOp::Neg => self.type_of(expr),
            },
            Expr::LogicAnd(_) | Expr::LogicOr(_) => Type::Bool,
            Expr::Call {
                module, callee, ..
            } => {
                let key = match module {
                    Some(m) => format!("{}.{}", m, callee),
                    None => callee.clone(),
                };
                self.functions
                    .get(&key)
                    .and_then(|s| s.return_type.clone())
                    .unwrap_or_else(|| unreachable!("sema ensures call returns a value"))
            }
            Expr::Input => Type::Deshnash,
            Expr::ArrayLit(elems) => {
                let inner = self.type_of(&elems[0]);
                Type::Array(Box::new(inner))
            }
            Expr::Index { target, .. } => match self.type_of(target) {
                Type::Array(inner) => *inner,
                _ => unreachable!("sema ensures index target is array"),
            },
            Expr::Baram(_) => Type::Terah,
            Expr::ParseTerah(_) => Type::Terah,
            Expr::ParseDaqosh(_) => Type::Daqosh,
            Expr::ToTerah(_) => Type::Terah,
            Expr::ToDaqosh(_) => Type::Daqosh,
            Expr::Pop(name) => match self.lookup(name).expect("sema ensures defined") {
                Type::Array(inner) => *inner,
                _ => unreachable!("sema ensures pop target is array"),
            },
            Expr::StructLit { name, .. } => Type::Struct(name.clone()),
            Expr::FieldAccess { target, field } => {
                let target_ty = self.type_of(target);
                let struct_name = match target_ty {
                    Type::Struct(n) => n,
                    _ => unreachable!("sema ensures field access on struct"),
                };
                let def = self.structs.get(&struct_name).expect("sema");
                def.fields
                    .iter()
                    .find(|f| f.name == *field)
                    .expect("sema ensures field exists")
                    .ty
                    .clone()
            }
        }
    }

    // --- scope management ---
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

    // --- output helpers ---
    fn write(&mut self, s: &str) {
        self.out.push_str(s);
    }
    fn writeln(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }
    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push_str("    ");
        }
    }
}

/// Mangled C name for a (possibly module-qualified) Mott function.
/// User-level functions emit as-is so `kort` becomes `int main(void)`
/// and `add(2, 3)` stays `add(2, 3)`. Module functions get a
/// `mott_<module>_` prefix to avoid clashing with libc symbols
/// (`sqrt`, `pow`, `sin` would collide otherwise).
fn c_func_name(module: &Option<String>, name: &str) -> String {
    match module {
        Some(m) => format!("mott_{}_{}", m, name),
        None => name.to_string(),
    }
}

/// Sort structs so each comes after its by-value dependencies. Sema has
/// already verified no cycles, so DFS is safe. Arrays don't count as
/// value-dependencies — `[T]` is heap-indirect.
fn topo_sort_structs(structs: &HashMap<String, StructDef>) -> Vec<StructDef> {
    fn visit(
        name: &str,
        structs: &HashMap<String, StructDef>,
        visited: &mut HashSet<String>,
        out: &mut Vec<StructDef>,
    ) {
        if visited.contains(name) {
            return;
        }
        visited.insert(name.to_string());
        let s = match structs.get(name) {
            Some(s) => s,
            None => return,
        };
        for field in &s.fields {
            if let Type::Struct(dep) = &field.ty {
                visit(dep, structs, visited, out);
            }
        }
        out.push(s.clone());
    }
    let mut visited = HashSet::new();
    let mut out = Vec::new();
    // Sort names for deterministic output (HashMap iter is random-ish).
    let mut names: Vec<&String> = structs.keys().collect();
    names.sort();
    for name in names {
        visit(name, structs, &mut visited, &mut out);
    }
    out
}

/// Walk every type used in the program and collect struct names that
/// appear as the element of an array. Each such name needs its own
/// `mott_arr_<Name>` machinery in the emitted C.
fn collect_struct_array_uses(program: &Program) -> Vec<String> {
    fn walk_type(t: &Type, out: &mut HashSet<String>) {
        match t {
            Type::Array(inner) => {
                if let Type::Struct(name) = inner.as_ref() {
                    out.insert(name.clone());
                }
                walk_type(inner, out);
            }
            _ => {}
        }
    }
    fn walk_expr(e: &Expr, out: &mut HashSet<String>) {
        match e {
            Expr::Binary { left, right, .. } => {
                walk_expr(left, out);
                walk_expr(right, out);
            }
            Expr::Unary { expr, .. } => walk_expr(expr, out),
            Expr::LogicAnd(es) | Expr::LogicOr(es) => {
                for e in es {
                    walk_expr(e, out);
                }
            }
            Expr::Call { args, .. } => {
                for a in args {
                    walk_expr(a, out);
                }
            }
            Expr::ArrayLit(es) => {
                for e in es {
                    walk_expr(e, out);
                }
            }
            Expr::Index { target, index } => {
                walk_expr(target, out);
                walk_expr(index, out);
            }
            Expr::Baram(e)
            | Expr::ParseTerah(e)
            | Expr::ParseDaqosh(e)
            | Expr::ToTerah(e)
            | Expr::ToDaqosh(e) => walk_expr(e, out),
            Expr::FieldAccess { target, .. } => walk_expr(target, out),
            Expr::StructLit { fields, .. } => {
                for (_, fe) in fields {
                    walk_expr(fe, out);
                }
            }
            Expr::String(parts) => {
                for p in parts {
                    if let StringPart::Interpolation(e) = p {
                        walk_expr(e, out);
                    }
                }
            }
            _ => {}
        }
    }
    fn walk_stmt(s: &Stmt, out: &mut HashSet<String>) {
        match s {
            Stmt::Let { ty, value, .. } => {
                if let Some(t) = ty {
                    walk_type(t, out);
                }
                if let Some(v) = value {
                    walk_expr(v, out);
                }
            }
            Stmt::Assign { value, .. } => walk_expr(value, out),
            Stmt::IndexAssign { index, value, .. } => {
                walk_expr(index, out);
                walk_expr(value, out);
            }
            Stmt::FieldAssign { value, .. } => walk_expr(value, out),
            Stmt::If {
                cond,
                then_block,
                else_block,
            } => {
                walk_expr(cond, out);
                for s in &then_block.stmts {
                    walk_stmt(s, out);
                }
                if let Some(eb) = else_block {
                    for s in &eb.stmts {
                        walk_stmt(s, out);
                    }
                }
            }
            Stmt::While { cond, body } => {
                walk_expr(cond, out);
                for s in &body.stmts {
                    walk_stmt(s, out);
                }
            }
            Stmt::ForEach { iter, body, .. } => {
                match iter {
                    IterSource::Array(e) => walk_expr(e, out),
                    IterSource::Range { start, end } => {
                        walk_expr(start, out);
                        walk_expr(end, out);
                    }
                }
                for s in &body.stmts {
                    walk_stmt(s, out);
                }
            }
            Stmt::Return(Some(e)) | Stmt::Print(e) | Stmt::ExprStmt(e) => walk_expr(e, out),
            Stmt::Push { value, .. } => walk_expr(value, out),
            _ => {}
        }
    }

    let mut set: HashSet<String> = HashSet::new();
    for item in &program.items {
        match item {
            Item::Function(f) => {
                for p in &f.params {
                    walk_type(&p.ty, &mut set);
                }
                if let Some(rt) = &f.return_type {
                    walk_type(rt, &mut set);
                }
                if let Some(body) = &f.body {
                    for s in &body.stmts {
                        walk_stmt(s, &mut set);
                    }
                }
            }
            Item::Struct(s) => {
                for f in &s.fields {
                    walk_type(&f.ty, &mut set);
                }
            }
            Item::Import { .. } => {}
        }
    }
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    v
}

/// If `b` is a single-stmt block containing only an `If`, return its pieces.
/// This is the shape the parser produces for `vusht nagah sanna` — letting
/// the backend re-emit `else if` instead of `else { if ... }`.
fn as_chained_if(b: &Block) -> Option<(&Expr, &Block, Option<&Block>)> {
    if b.stmts.len() != 1 {
        return None;
    }
    match &b.stmts[0] {
        Stmt::If {
            cond,
            then_block,
            else_block,
        } => Some((cond, then_block, else_block.as_ref())),
        _ => None,
    }
}

fn type_to_c(t: &Type) -> String {
    match t {
        Type::Terah => "int64_t".into(),
        Type::Daqosh => "double".into(),
        Type::Bool => "bool".into(),
        Type::Deshnash => "mott_str".into(),
        Type::Struct(name) => name.clone(),
        Type::Array(inner) => match inner.as_ref() {
            Type::Terah => "mott_arr_terah".into(),
            Type::Daqosh => "mott_arr_daqosh".into(),
            Type::Bool => "mott_arr_bool".into(),
            Type::Deshnash => "mott_arr_deshnash".into(),
            // Per-struct array typedefs are emitted by the program prelude
            // (see `emit_struct_array_machinery`).
            Type::Struct(name) => format!("mott_arr_{}", name),
            Type::Array(_) => "mott_arr_nested".into(), // unreachable — rejected by sema
        },
    }
}

/// Names are returned as owned strings now that user-defined struct
/// names participate. The static-str cases still resolve cheaply.
fn array_ctor_name(t: &Type) -> String {
    match t {
        Type::Terah => "mott_arr_terah_new".into(),
        Type::Daqosh => "mott_arr_daqosh_new".into(),
        Type::Bool => "mott_arr_bool_new".into(),
        Type::Deshnash => "mott_arr_deshnash_new".into(),
        Type::Struct(name) => format!("mott_arr_{}_new", name),
        Type::Array(_) => "mott_arr_nested_new".into(),
    }
}

fn array_push_name(t: &Type) -> String {
    match t {
        Type::Terah => "mott_arr_terah_push".into(),
        Type::Daqosh => "mott_arr_daqosh_push".into(),
        Type::Bool => "mott_arr_bool_push".into(),
        Type::Deshnash => "mott_arr_deshnash_push".into(),
        Type::Struct(name) => format!("mott_arr_{}_push", name),
        Type::Array(_) => "mott_arr_nested_push".into(),
    }
}

fn array_pop_name(t: &Type) -> String {
    match t {
        Type::Terah => "mott_arr_terah_pop".into(),
        Type::Daqosh => "mott_arr_daqosh_pop".into(),
        Type::Bool => "mott_arr_bool_pop".into(),
        Type::Deshnash => "mott_arr_deshnash_pop".into(),
        Type::Struct(name) => format!("mott_arr_{}_pop", name),
        Type::Array(_) => "mott_arr_nested_pop".into(),
    }
}

/// Zero-value C expression for a mott type. Used to initialize typed
/// `xilit` declarations that omit an initializer.
fn zero_value(t: &Type) -> String {
    match t {
        Type::Terah => "((int64_t)0)".into(),
        Type::Daqosh => "0.0".into(),
        Type::Bool => "false".into(),
        Type::Deshnash => "MOTT_STR_LIT(\"\")".into(),
        Type::Array(inner) => {
            let ctor = array_ctor_name(inner);
            format!("{}(0, NULL)", ctor)
        }
        // C compound literal `(Foo){0}` zero-initializes every field
        // recursively via the fact that designated initializers default
        // any unmentioned field to zero. Works for any struct shape.
        Type::Struct(name) => format!("(({}){{0}})", name),
    }
}

fn bin_op_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => " + ",
        BinOp::Sub => " - ",
        BinOp::Mul => " * ",
        BinOp::Div => " / ",
        BinOp::Mod => " % ",
        BinOp::Eq => " == ",
        BinOp::NotEq => " != ",
        BinOp::Lt => " < ",
        BinOp::Le => " <= ",
        BinOp::Gt => " > ",
        BinOp::Ge => " >= ",
    }
}

/// Render `s` as a C double-quoted string literal. UTF-8 bytes outside
/// printable ASCII go through as `\xNN` — works because mott source is
/// UTF-8 and we never split a multibyte sequence.
fn c_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\\""),
            b'\n' => out.push_str("\\n"),
            b'\t' => out.push_str("\\t"),
            b'\r' => out.push_str("\\r"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                // Hex-escape followed by a string-break prevents the next
                // ASCII digit from being absorbed into the \x escape.
                let _ = write!(out, "\\x{:02x}\"\"", b);
            }
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::sema;

    /// Compile through the full pipeline: lex → parse → sema → codegen.
    /// Returns the generated C, or the first error from any stage.
    /// All the validation tests this used to host moved to `sema::tests`;
    /// what's left here is just emit-shape verification.
    fn compile(src: &str) -> Result<String> {
        let tokens = Lexer::new(src).tokenize()?;
        let program = Parser::new(tokens).parse()?;
        sema::check(&program)?;
        CBackend.emit(&program)
    }

    fn compile_ok(src: &str) -> String {
        compile(src).expect("compilation should succeed")
    }

    #[test]
    fn hello_world_produces_main() {
        let c = compile_ok("fnc kort() { yazde(\"Salam\"); }");
        assert!(c.contains("int main(void)"), "got:\n{}", c);
        assert!(c.contains("mott_yazde_deshnash"), "got:\n{}", c);
        assert!(c.contains("MOTT_STR_LIT(\"Salam\")"), "got:\n{}", c);
        assert!(c.contains("return 0;"), "got:\n{}", c);
    }

    #[test]
    fn int_let_and_arithmetic() {
        let c = compile_ok("fnc kort() { xilit x = 5; xilit y: terah = x + 3; yazde(y); }");
        assert!(c.contains("int64_t x ="));
        assert!(c.contains("int64_t y ="));
        assert!(c.contains("mott_yazde_terah(y)"));
    }

    #[test]
    fn interpolation_emits_str_build() {
        let c = compile_ok("fnc kort() { xilit x = 5; yazde(\"x = {x}\"); }");
        assert!(c.contains("mott_str_build"), "got:\n{}", c);
        assert!(c.contains("mott_str_from_terah(x)"), "got:\n{}", c);
        assert!(c.contains("MOTT_STR_LIT(\"x = \")"), "got:\n{}", c);
    }

    #[test]
    fn if_else_and_while() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit i: terah = 0;
    cqachunna (i < 3) {
        nagah sanna (i == 1) {
            yazde("one");
        } vusht {
            yazde("other");
        }
        i = i + 1;
    }
}
"#,
        );
        assert!(c.contains("while ("));
        assert!(c.contains("if ("));
        assert!(c.contains("} else {"));
    }

    #[test]
    fn khi_nagah_sanna_emits_else_if() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit x: terah = 1;
    nagah sanna (x == 1) {
        yazde("one");
    } vusht nagah sanna (x == 2) {
        yazde("two");
    } vusht nagah sanna (x == 3) {
        yazde("three");
    } vusht {
        yazde("other");
    }
}
"#,
        );
        assert!(c.contains("} else if ("), "expected idiomatic `else if`, got:\n{}", c);
        assert_eq!(c.matches("} else {").count(), 1, "got:\n{}", c);
    }

    #[test]
    fn logic_and_becomes_cc_and() {
        let c = compile_ok(
            "fnc kort() { xilit x: terah = 5; nagah sanna (x > 0 a, x < 10 a) { yazde(\"ok\"); } }",
        );
        assert!(c.contains(" && "), "got:\n{}", c);
    }

    #[test]
    fn logic_or_becomes_cc_or() {
        let c = compile_ok(
            "fnc kort() { xilit x: terah = 5; nagah sanna (x < 0 ya x > 10) { yazde(\"out\"); } }",
        );
        assert!(c.contains(" || "), "got:\n{}", c);
    }

    #[test]
    fn function_with_params_emits_forward_decl() {
        let c = compile_ok(
            r#"
fnc add(x: terah, y: terah) -> terah {
    yuxadalo x + y;
}
fnc kort() {
    xilit z = add(2, 3);
    yazde(z);
}
"#,
        );
        assert!(c.contains("int64_t add(int64_t x, int64_t y);"), "got:\n{}", c);
        assert!(c.contains("int64_t add(int64_t x, int64_t y) {"), "got:\n{}", c);
        assert!(c.contains("add(((int64_t)2LL), ((int64_t)3LL))"), "got:\n{}", c);
    }

    #[test]
    fn sac_and_khida_emit_break_continue() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit i: terah = 0;
    cqachunna (i < 10) {
        nagah sanna (i == 5) {
            sac;
        }
        i = i + 1;
        khida;
    }
}
"#,
        );
        assert!(c.contains("break;"), "got:\n{}", c);
        assert!(c.contains("continue;"), "got:\n{}", c);
    }

    #[test]
    fn string_equality_lowers_to_runtime_call() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit name: deshnash = "salam"
    nagah sanna name == "salam" {
        yazde("hi")
    }
}
"#,
        );
        assert!(c.contains("mott_str_eq("), "got:\n{}", c);
        assert!(!c.contains("name == MOTT_STR_LIT"), "got:\n{}", c);
    }

    #[test]
    fn string_inequality_emits_negated_runtime_call() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit name: deshnash = "salam"
    nagah sanna name != "marshalla" {
        yazde("not marshalla")
    }
}
"#,
        );
        assert!(c.contains("(!mott_str_eq("), "got:\n{}", c);
    }

    #[test]
    fn esha_emits_mott_input_call_typed_as_deshnash() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit line: deshnash = esha()
    yazde(line)
}
"#,
        );
        assert!(c.contains("mott_input()"), "got:\n{}", c);
        assert!(c.contains("mott_str line = "), "got:\n{}", c);
    }

    #[test]
    fn array_literal_emits_runtime_constructor() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = [1, 2, 3]
    yazde(baram(nums))
}
"#,
        );
        assert!(c.contains("mott_arr_terah nums = "), "got:\n{}", c);
        assert!(c.contains("mott_arr_terah_new(3"), "got:\n{}", c);
        assert!(c.contains("(int64_t)(nums.len)"), "got:\n{}", c);
    }

    #[test]
    fn for_each_array_emits_indexed_loop() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = [10, 20, 30]
    yallalc x chu nums {
        yazde(x)
    }
}
"#,
        );
        assert!(c.contains("__mott_arr.len"), "got:\n{}", c);
        assert!(c.contains("int64_t x = __mott_arr.data[__mott_i]"), "got:\n{}", c);
    }

    #[test]
    fn for_each_range_emits_counting_loop() {
        let c = compile_ok(
            r#"
fnc kort() {
    yallalc i chu 0..5 {
        yazde(i)
    }
}
"#,
        );
        assert!(c.contains("for (int64_t i = "), "got:\n{}", c);
        assert!(c.contains("i++"), "got:\n{}", c);
    }

    #[test]
    fn baram_on_string_works() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit s: deshnash = "salam"
    yazde(baram(s))
}
"#,
        );
        assert!(c.contains("(int64_t)(s.len)"), "got:\n{}", c);
    }

    #[test]
    fn array_index_assignment() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = [1, 2, 3]
    nums[0] = 99
}
"#,
        );
        assert!(c.contains("nums.data["), "got:\n{}", c);
    }

    #[test]
    fn parse_terah_emits_runtime_call() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit s: deshnash = "42"
    xilit n: terah = parse_terah(s)
    yazde(n)
}
"#,
        );
        assert!(c.contains("mott_parse_terah(s)"), "got:\n{}", c);
    }

    #[test]
    fn parse_daqosh_emits_runtime_call() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit s: deshnash = "3.14"
    xilit x: daqosh = parse_daqosh(s)
    yazde(x)
}
"#,
        );
        assert!(c.contains("mott_parse_daqosh(s)"), "got:\n{}", c);
    }

    #[test]
    fn to_terah_emits_int64_cast() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit x: daqosh = 3.7
    xilit n: terah = to_terah(x)
    yazde(n)
}
"#,
        );
        assert!(c.contains("((int64_t)(x))"), "got:\n{}", c);
    }

    #[test]
    fn to_daqosh_emits_double_cast() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit n: terah = 42
    xilit x: daqosh = to_daqosh(n)
    yazde(x)
}
"#,
        );
        assert!(c.contains("((double)(n))"), "got:\n{}", c);
    }

    #[test]
    fn push_emits_runtime_call_with_address() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = [1, 2]
    push(nums, 3)
}
"#,
        );
        assert!(c.contains("mott_arr_terah_push(&nums, "), "got:\n{}", c);
    }

    #[test]
    fn pop_emits_runtime_call_with_address() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = [1, 2, 3]
    xilit last: terah = pop(nums)
    yazde(last)
}
"#,
        );
        assert!(c.contains("mott_arr_terah_pop(&nums)"), "got:\n{}", c);
    }

    #[test]
    fn empty_array_literal_with_annotation_emits_zero_new() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah] = []
    push(nums, 1)
}
"#,
        );
        assert!(c.contains("mott_arr_terah_new(0, NULL)"), "got:\n{}", c);
    }

    #[test]
    fn uninit_terah_decl_zero_initializes() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit x: terah
    yazde(x)
}
"#,
        );
        assert!(c.contains("int64_t x = ((int64_t)0);"), "got:\n{}", c);
    }

    #[test]
    fn uninit_deshnash_decl_emits_empty_string() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit s: deshnash
    yazde(s)
}
"#,
        );
        assert!(
            c.contains(r#"mott_str s = MOTT_STR_LIT("");"#),
            "got:\n{}",
            c
        );
    }

    #[test]
    fn uninit_array_decl_zero_len_constructor() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit nums: [terah]
    push(nums, 42)
}
"#,
        );
        assert!(
            c.contains("mott_arr_terah nums = mott_arr_terah_new(0, NULL);"),
            "got:\n{}",
            c
        );
        assert!(c.contains("mott_arr_terah_push(&nums, "), "got:\n{}", c);
    }

    #[test]
    fn interpolation_with_arithmetic_expression() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit x: terah = 10
    xilit y: terah = 3
    yazde("diff: {x - y}")
}
"#,
        );
        assert!(c.contains("mott_str_from_terah("), "got:\n{}", c);
        assert!(c.contains("(x - y)"), "got:\n{}", c);
    }

    #[test]
    fn interpolation_with_function_call() {
        let c = compile_ok(
            r#"
fnc sq(n: terah) -> terah {
    yuxadalo n * n
}
fnc kort() {
    xilit x: terah = 5
    yazde("x^2 = {sq(x)}")
}
"#,
        );
        assert!(c.contains("mott_str_from_terah(sq("), "got:\n{}", c);
    }

    #[test]
    fn interpolation_with_comparison_uses_bool_converter() {
        let c = compile_ok(
            r#"
fnc kort() {
    xilit x: terah = 5
    xilit y: terah = 3
    yazde("x > y? {x > y}")
}
"#,
        );
        assert!(c.contains("mott_str_from_bool("), "got:\n{}", c);
    }

    #[test]
    fn struct_typedef_emitted() {
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point = Point { x: 3, y: 5 }
}
"#,
        );
        assert!(c.contains("typedef struct Point {"), "got:\n{}", c);
        assert!(c.contains("int64_t x;"), "got:\n{}", c);
        assert!(c.contains("int64_t y;"), "got:\n{}", c);
        assert!(c.contains("} Point;"), "got:\n{}", c);
    }

    #[test]
    fn struct_literal_emits_compound_literal_in_decl_order() {
        // Field initializer order in source uses `y, x` but declaration
        // is `x, y`. Codegen emits in declaration order so the C output
        // is stable regardless of source ordering.
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point = Point { y: 5, x: 3 }
    yazde("{p.x}")
}
"#,
        );
        // Both .x = 3 and .y = 5 should appear, in declaration order.
        let x_pos = c.find(".x = ").unwrap();
        let y_pos = c.find(".y = ").unwrap();
        assert!(x_pos < y_pos, "fields should be in decl order, got:\n{}", c);
    }

    #[test]
    fn field_access_emits_member_access() {
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point = Point { x: 3, y: 5 }
    yazde("{p.x}")
}
"#,
        );
        assert!(c.contains("p.x"), "got:\n{}", c);
    }

    #[test]
    fn field_assignment_emits_member_assign() {
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point = Point { x: 3, y: 5 }
    p.x = 10
}
"#,
        );
        assert!(c.contains("p.x = "), "got:\n{}", c);
    }

    #[test]
    fn array_of_struct_emits_per_struct_machinery() {
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit pts: [Point] = []
    push(pts, Point { x: 1, y: 2 })
}
"#,
        );
        assert!(
            c.contains("typedef struct { Point *data; size_t len; size_t cap; } mott_arr_Point;"),
            "got:\n{}",
            c
        );
        assert!(c.contains("mott_arr_Point_new"), "got:\n{}", c);
        assert!(c.contains("mott_arr_Point_push"), "got:\n{}", c);
    }

    #[test]
    fn struct_typedefs_emitted_in_topological_order() {
        // Outer depends on Inner; Inner must be defined first.
        let c = compile_ok(
            r#"
kep Outer { i: Inner }
kep Inner { v: terah }
fnc kort() {
    xilit o: Outer = Outer { i: Inner { v: 5 } }
}
"#,
        );
        let inner_pos = c.find("typedef struct Inner").unwrap();
        let outer_pos = c.find("typedef struct Outer").unwrap();
        assert!(
            inner_pos < outer_pos,
            "Inner must come before Outer, got:\n{}",
            c
        );
    }

    #[test]
    fn uninit_struct_decl_zero_initializes() {
        let c = compile_ok(
            r#"
kep Point { x: terah, y: terah }
fnc kort() {
    xilit p: Point
    p.x = 5
}
"#,
        );
        assert!(c.contains("Point p = ((Point){0});"), "got:\n{}", c);
    }

    #[test]
    fn interpolation_with_deshnash_expression_skips_converter() {
        let c = compile_ok(
            r#"
fnc identity(s: deshnash) -> deshnash {
    yuxadalo s
}
fnc kort() {
    xilit name: deshnash = "World"
    yazde("hi, {identity(name)}!")
}
"#,
        );
        assert!(c.contains("identity("), "got:\n{}", c);
        assert!(!c.contains("mott_str_from_deshnash"), "got:\n{}", c);
    }
}
