use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::*;
use crate::env::Environment;
use crate::error::RunError;
use crate::value::{DictData, EnvRef, FunctionValue, RangeValue, Value};

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub comparisons: u64,
    pub assignments: u64,
    pub function_calls: u64,
    pub peak_element_count: u64,
}

pub struct Interpreter {
    pub output: RefCell<Vec<String>>,
    pub stats: RefCell<Stats>,
    input_lines: Vec<String>,
    input_pos: RefCell<usize>,
    depth: RefCell<usize>,
    current_line: RefCell<usize>,
    globals: EnvRef,
    loop_depth: RefCell<usize>,
}

const MAX_DEPTH: usize = 500;

impl Interpreter {
    pub fn new(input: &str) -> Self {
        let input_lines: Vec<String> = if input.is_empty() {
            Vec::new()
        } else {
            input.split('\n').map(|s| s.to_string()).collect()
        };
        let globals = Rc::new(RefCell::new(Environment::new()));
        Interpreter {
            output: RefCell::new(Vec::new()),
            stats: RefCell::new(Stats::default()),
            input_lines,
            input_pos: RefCell::new(0),
            depth: RefCell::new(0),
            current_line: RefCell::new(0),
            globals,
            loop_depth: RefCell::new(0),
        }
    }

    pub fn run(&self, program: &Program) -> Result<(), RunError> {
        let env = self.globals.clone();
        for stmt in &program.stmts {
            self.exec_stmt(stmt, env.clone())?;
        }
        Ok(())
    }

    fn err(&self, line: usize, message: impl Into<String>) -> RunError {
        RunError {
            line,
            message: message.into(),
        }
    }

    fn set_line(&self, line: usize) {
        *self.current_line.borrow_mut() = line;
    }

    fn bump_peak(&self, count: usize) {
        let mut stats = self.stats.borrow_mut();
        if (count as u64) > stats.peak_element_count {
            stats.peak_element_count = count as u64;
        }
    }

    fn count_all_elements(&self, env: &EnvRef) -> usize {
        let mut total = 0;
        let mut current = Some(env.clone());
        while let Some(e) = current {
            let borrow = e.borrow();
            for v in borrow.vars.values() {
                total += self.value_size(v);
            }
            current = borrow.parent();
        }
        total
    }

    fn value_size(&self, v: &Value) -> usize {
        match v {
            Value::List(l) => l.borrow().len(),
            Value::Dict(d) => d.borrow().entries.len(),
            Value::Set(s) => s.borrow().len(),
            Value::Tuple(t) => t.len(),
            Value::Heap(h) => h.borrow().len(),
            _ => 0,
        }
    }

    fn exec_block(&self, block: &Block, env: EnvRef) -> Result<(), RunError> {
        for stmt in &block.stmts {
            self.exec_stmt(stmt, env.clone())?;
        }
        Ok(())
    }

    fn exec_stmt(&self, stmt: &Stmt, env: EnvRef) -> Result<(), RunError> {
        match stmt {
            Stmt::VarDecl(v) => self.exec_var_decl(v, env),
            Stmt::Assign(a) => self.exec_assign(a, env),
            Stmt::If(s) => self.exec_if(s, env),
            Stmt::While(s) => self.exec_while(s, env),
            Stmt::For(s) => self.exec_for(s, env),
            Stmt::Def(s) => self.exec_def(s, env),
            Stmt::Return(s) => self.exec_return(s, env),
            Stmt::Break(line) => {
                if *self.loop_depth.borrow() == 0 {
                    return Err(self.err(*line, "break outside loop"));
                }
                Err(self.err(*line, "__break__"))
            }
            Stmt::Continue(line) => {
                if *self.loop_depth.borrow() == 0 {
                    return Err(self.err(*line, "continue outside loop"));
                }
                Err(self.err(*line, "__continue__"))
            }
            Stmt::ExprStmt(e) => {
                self.eval_expr(&e.expr, env)?;
                Ok(())
            }
        }
    }

    fn exec_var_decl(&self, v: &VarDecl, env: EnvRef) -> Result<(), RunError> {
        self.set_line(v.line);
        let val = self.eval_expr(&v.init, env.clone())?;
        self.check_type(&v.type_name, &val, v.line)?;
        env.borrow_mut().define(v.name.clone(), val);
        self.stats.borrow_mut().assignments += 1;
        self.update_peak(&env);
        Ok(())
    }

    fn exec_assign(&self, a: &Assign, env: EnvRef) -> Result<(), RunError> {
        self.set_line(a.line);
        let rhs = self.eval_expr(&a.value, env.clone())?;
        match &a.target {
            Expr::Var(name) => {
                let new_val = if a.op == AssignOp::Assign {
                    rhs
                } else {
                    let old = self.lookup_var(name, &env, a.line)?;
                    self.apply_compound(a.op, old, rhs, a.line)?
                };
                if !self.set_var(name, new_val, &env) {
                    return Err(self.err(a.line, format!("undefined variable '{}'", name)));
                }
                self.stats.borrow_mut().assignments += 1;
            }
            Expr::Index(idx) => {
                let obj = self.eval_expr(&idx.object, env.clone())?;
                let key = self.eval_expr(&idx.index, env.clone())?;
                let new_val = if a.op == AssignOp::Assign {
                    rhs
                } else {
                    let old = self.index_get(&obj, &key, a.line)?;
                    self.apply_compound(a.op, old, rhs, a.line)?
                };
                self.index_set(&obj, &key, new_val, a.line)?;
                self.stats.borrow_mut().assignments += 1;
            }
            _ => {
                return Err(self.err(a.line, "invalid assignment target"));
            }
        }
        self.update_peak(&env);
        Ok(())
    }

    fn apply_compound(
        &self,
        op: AssignOp,
        old: Value,
        rhs: Value,
        line: usize,
    ) -> Result<Value, RunError> {
        let bin = match op {
            AssignOp::AddAssign => BinOp::Add,
            AssignOp::SubAssign => BinOp::Sub,
            AssignOp::MulAssign => BinOp::Mul,
            AssignOp::DivAssign => BinOp::Div,
            AssignOp::Assign => return Ok(rhs),
        };
        self.binary_op(bin, old, rhs, line)
    }

    fn exec_if(&self, s: &IfStmt, env: EnvRef) -> Result<(), RunError> {
        for (cond, body) in &s.branches {
            let c = self.eval_expr(cond, env.clone())?;
            if !matches!(c, Value::Bool(_)) {
                return Err(self.err(s.line, "condition must be OrionBool"));
            }
            if c.is_truthy() {
                return self.exec_block(body, env);
            }
        }
        if let Some(body) = &s.else_block {
            return self.exec_block(body, env);
        }
        Ok(())
    }

    fn exec_while(&self, s: &WhileStmt, env: EnvRef) -> Result<(), RunError> {
        loop {
            let c = self.eval_expr(&s.cond, env.clone())?;
            if !matches!(c, Value::Bool(_)) {
                return Err(self.err(s.line, "condition must be OrionBool"));
            }
            if !c.is_truthy() {
                break;
            }
            *self.loop_depth.borrow_mut() += 1;
            let r = self.exec_block(&s.body, env.clone());
            *self.loop_depth.borrow_mut() -= 1;
            match r {
                Ok(()) => {}
                Err(e) if e.message == "__break__" => break,
                Err(e) if e.message == "__continue__" => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn exec_for(&self, s: &ForStmt, env: EnvRef) -> Result<(), RunError> {
        let iterable = self.eval_expr(&s.iter, env.clone())?;
        let items = self.iter_items(&iterable, s.vars.len(), s.line)?;
        for item in items {
            if s.vars.len() == 1 {
                env.borrow_mut().define(s.vars[0].clone(), item);
                self.stats.borrow_mut().assignments += 1;
            } else if let Value::Tuple(t) = &item {
                if t.len() != 2 {
                    return Err(self.err(s.line, "dict item must be a 2-tuple"));
                }
                env.borrow_mut().define(s.vars[0].clone(), t[0].clone());
                env.borrow_mut().define(s.vars[1].clone(), t[1].clone());
                self.stats.borrow_mut().assignments += 2;
            } else {
                return Err(self.err(s.line, "dict iteration expects key-value pair"));
            }
            *self.loop_depth.borrow_mut() += 1;
            let r = self.exec_block(&s.body, env.clone());
            *self.loop_depth.borrow_mut() -= 1;
            match r {
                Ok(()) => {}
                Err(e) if e.message == "__break__" => break,
                Err(e) if e.message == "__continue__" => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    fn iter_items(&self, v: &Value, nvars: usize, line: usize) -> Result<Vec<Value>, RunError> {
        match (nvars, v) {
            (1, Value::List(l)) => Ok(l.borrow().clone()),
            (1, Value::Range(r)) => {
                let mut out = Vec::new();
                let mut i = r.start;
                while i < r.end {
                    out.push(Value::Int(i));
                    i += 1;
                }
                Ok(out)
            }
            (2, Value::Dict(d)) => Ok(d.borrow().items()),
            (1, _) => Err(self.err(
                line,
                "single-variable for requires OrionList or OrionRange",
            )),
            (2, _) => Err(self.err(line, "two-variable for requires OrionDict")),
            _ => Err(self.err(line, "invalid for loop")),
        }
    }

    fn exec_def(&self, s: &DefStmt, env: EnvRef) -> Result<(), RunError> {
        let func = FunctionValue {
            name: s.name.clone(),
            params: s.params.clone(),
            return_type: s.return_type.clone(),
            body: s.body.clone(),
            closure: env.clone(),
        };
        env.borrow_mut()
            .define(s.name.clone(), Value::Function(Rc::new(func)));
        self.stats.borrow_mut().assignments += 1;
        Ok(())
    }

    fn exec_return(&self, s: &ReturnStmt, env: EnvRef) -> Result<(), RunError> {
        let v = self.eval_expr(&s.value, env)?;
        Err(self.err(s.line, format!("__return__{}", encode_value(&v))))
    }

    fn eval_expr(&self, e: &Expr, env: EnvRef) -> Result<Value, RunError> {
        match e {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::None => Ok(Value::None),
            Expr::Var(name) => self.lookup_var(name, &env, 0),
            Expr::BinOp(b) => self.eval_binop(b, env),
            Expr::UnOp(u) => self.eval_unop(u, env),
            Expr::Call(c) => self.eval_call(c, env),
            Expr::Index(i) => {
                let obj = self.eval_expr(&i.object, env.clone())?;
                let key = self.eval_expr(&i.index, env.clone())?;
                self.index_get(&obj, &key, i.line)
            }
            Expr::ListLit(items, _) => {
                let mut vals = Vec::new();
                for it in items {
                    vals.push(self.eval_expr(it, env.clone())?);
                }
                self.update_peak(&env);
                Ok(Value::List(Rc::new(RefCell::new(vals))))
            }
            Expr::DictLit(pairs, _) => {
                let mut d = DictData::new();
                for (k, v) in pairs {
                    let kv = self.eval_expr(k, env.clone())?;
                    let vv = self.eval_expr(v, env.clone())?;
                    d.set(kv, vv);
                }
                self.update_peak(&env);
                Ok(Value::Dict(Rc::new(RefCell::new(d))))
            }
            Expr::MethodCall(m) => self.eval_method_call(m, env),
        }
    }

    fn eval_method_call(&self, m: &MethodCallExpr, env: EnvRef) -> Result<Value, RunError> {
        let obj = self.eval_expr(&m.object, env.clone())?;
        let mut args = Vec::new();
        for a in &m.args {
            args.push(self.eval_expr(a, env.clone())?);
        }
        match (&obj, m.method.as_str()) {
            (Value::List(l), "Add") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "List.Add expects 1 argument"));
                }
                l.borrow_mut().push(args[0].clone());
                self.update_peak(&env);
                Ok(Value::None)
            }
            (Value::List(l), "Del") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "List.Del expects 1 argument"));
                }
                let idx = self.expect_int(&args[0], m.line)?;
                let len = l.borrow().len();
                let i = self.normalize_index(idx, len, m.line)?;
                l.borrow_mut().remove(i);
                Ok(Value::None)
            }
            (Value::List(l), "AddFront") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "List.AddFront expects 1 argument"));
                }
                l.borrow_mut().insert(0, args[0].clone());
                self.update_peak(&env);
                Ok(Value::None)
            }
            (Value::List(l), "DelFront") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "List.DelFront expects no arguments"));
                }
                let len = l.borrow().len();
                if len == 0 {
                    return Err(self.err(m.line, "DelFront on empty list"));
                }
                let v = l.borrow_mut().remove(0);
                Ok(v)
            }
            (Value::Dict(d), "Keys") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "Dict.Keys expects no arguments"));
                }
                Ok(Value::List(Rc::new(RefCell::new(d.borrow().keys()))))
            }
            (Value::Dict(d), "Values") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "Dict.Values expects no arguments"));
                }
                Ok(Value::List(Rc::new(RefCell::new(d.borrow().values()))))
            }
            (Value::Dict(d), "Items") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "Dict.Items expects no arguments"));
                }
                Ok(Value::List(Rc::new(RefCell::new(d.borrow().items()))))
            }
            (Value::Set(s), "Add") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "Set.Add expects 1 argument"));
                }
                let exists = s.borrow().iter().any(|x| x.value_eq(&args[0]));
                if !exists {
                    s.borrow_mut().push(args[0].clone());
                    self.update_peak(&env);
                }
                Ok(Value::None)
            }
            (Value::Set(s), "Del") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "Set.Del expects 1 argument"));
                }
                let pos = s.borrow().iter().position(|x| x.value_eq(&args[0]));
                if let Some(i) = pos {
                    s.borrow_mut().remove(i);
                }
                Ok(Value::None)
            }
            (Value::Heap(h), "Add") => {
                if args.len() != 1 {
                    return Err(self.err(m.line, "Heap.Add expects 1 argument"));
                }
                h.borrow_mut().push(args[0].clone());
                let mut i = h.borrow().len() - 1;
                while i > 0 {
                    let parent = (i - 1) / 2;
                    let cmp = {
                        self.stats.borrow_mut().comparisons += 1;
                        let b = h.borrow();
                        self.partial_cmp(&b[i], &b[parent], m.line)?
                    };
                    if cmp == std::cmp::Ordering::Less {
                        let mut b = h.borrow_mut();
                        b.swap(i, parent);
                        i = parent;
                    } else {
                        break;
                    }
                }
                self.update_peak(&env);
                Ok(Value::None)
            }
            (Value::Heap(h), "Del") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "Heap.Del expects no arguments"));
                }
                let n = h.borrow().len();
                if n == 0 {
                    return Err(self.err(m.line, "Del on empty heap"));
                }
                h.borrow_mut().swap(0, n - 1);
                let min = h.borrow_mut().pop().unwrap();
                if !h.borrow().is_empty() {
                    let mut i = 0;
                    loop {
                        let l = 2 * i + 1;
                        let r = 2 * i + 2;
                        let mut smallest = i;
                        if l < h.borrow().len() {
                            self.stats.borrow_mut().comparisons += 1;
                            let b = h.borrow();
                            if self.partial_cmp(&b[l], &b[smallest], m.line)?
                                == std::cmp::Ordering::Less
                            {
                                smallest = l;
                            }
                        }
                        if r < h.borrow().len() {
                            self.stats.borrow_mut().comparisons += 1;
                            let b = h.borrow();
                            if self.partial_cmp(&b[r], &b[smallest], m.line)?
                                == std::cmp::Ordering::Less
                            {
                                smallest = r;
                            }
                        }
                        if smallest == i {
                            break;
                        }
                        let mut b = h.borrow_mut();
                        b.swap(i, smallest);
                        i = smallest;
                    }
                }
                Ok(min)
            }
            (Value::Heap(h), "Peek") => {
                if !args.is_empty() {
                    return Err(self.err(m.line, "Heap.Peek expects no arguments"));
                }
                let len = h.borrow().len();
                if len == 0 {
                    return Err(self.err(m.line, "Peek on empty heap"));
                }
                let val = h.borrow()[0].clone();
                Ok(val)
            }
            _ => Err(self.err(m.line, format!(
                "method '{}' not supported for {}",
                m.method,
                obj.type_name()
            ))),
        }
    }

    fn eval_binop(&self, b: &BinOpExpr, env: EnvRef) -> Result<Value, RunError> {
        if b.op == BinOp::And {
            let l = self.eval_expr(&b.left, env.clone())?;
            if !matches!(l, Value::Bool(_)) {
                return Err(self.err(b.line, "left operand of 'and' must be OrionBool"));
            }
            if !l.is_truthy() {
                return Ok(Value::Bool(false));
            }
            let r = self.eval_expr(&b.right, env.clone())?;
            if !matches!(r, Value::Bool(_)) {
                return Err(self.err(b.line, "right operand of 'and' must be OrionBool"));
            }
            return Ok(r);
        }
        if b.op == BinOp::Or {
            let l = self.eval_expr(&b.left, env.clone())?;
            if !matches!(l, Value::Bool(_)) {
                return Err(self.err(b.line, "left operand of 'or' must be OrionBool"));
            }
            if l.is_truthy() {
                return Ok(Value::Bool(true));
            }
            let r = self.eval_expr(&b.right, env.clone())?;
            if !matches!(r, Value::Bool(_)) {
                return Err(self.err(b.line, "right operand of 'or' must be OrionBool"));
            }
            return Ok(r);
        }
        let l = self.eval_expr(&b.left, env.clone())?;
        let r = self.eval_expr(&b.right, env.clone())?;
        self.binary_op(b.op, l, r, b.line)
    }

    fn binary_op(&self, op: BinOp, l: Value, r: Value, line: usize) -> Result<Value, RunError> {
        match op {
            BinOp::Add => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(_), _) | (_, Value::Float(_)) => {
                    let a = l.as_float().unwrap();
                    let b = r.as_float().unwrap();
                    Ok(Value::Float(a + b))
                }
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Err(self.err(line, format!(
                    "operator '+' not supported for {} and {}",
                    l.type_name(),
                    r.type_name()
                ))),
            },
            BinOp::Sub => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                _ if l.as_float().is_some() && r.as_float().is_some() => {
                    Ok(Value::Float(l.as_float().unwrap() - r.as_float().unwrap()))
                }
                _ => Err(self.err(line, format!(
                    "operator '-' not supported for {} and {}",
                    l.type_name(),
                    r.type_name()
                ))),
            },
            BinOp::Mul => match (&l, &r) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Str(s), Value::Int(n)) => Ok(Value::Str(s.repeat(*n as usize))),
                (Value::Int(n), Value::Str(s)) => Ok(Value::Str(s.repeat(*n as usize))),
                _ if l.as_float().is_some() && r.as_float().is_some() => {
                    Ok(Value::Float(l.as_float().unwrap() * r.as_float().unwrap()))
                }
                _ => Err(self.err(line, format!(
                    "operator '*' not supported for {} and {}",
                    l.type_name(),
                    r.type_name()
                ))),
            },
            BinOp::Div => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b == 0 {
                        return Err(self.err(line, "division by zero"));
                    }
                    return Ok(Value::Float(*a as f64 / *b as f64));
                }
                if l.as_float().is_some() && r.as_float().is_some() {
                    let b = r.as_float().unwrap();
                    if b == 0.0 {
                        return Err(self.err(line, "division by zero"));
                    }
                    return Ok(Value::Float(l.as_float().unwrap() / b));
                }
                Err(self.err(line, format!(
                    "operator '/' not supported for {} and {}",
                    l.type_name(),
                    r.type_name()
                )))
            }
            BinOp::FloorDiv => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b == 0 {
                        return Err(self.err(line, "division by zero"));
                    }
                    return Ok(Value::Int(a.div_euclid(*b)));
                }
                Err(self.err(line, "operator '//' requires OrionInt operands"))
            }
            BinOp::Mod => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b == 0 {
                        return Err(self.err(line, "modulo by zero"));
                    }
                    return Ok(Value::Int(a.rem_euclid(*b)));
                }
                Err(self.err(line, "operator '%' requires OrionInt operands"))
            }
            BinOp::Pow => {
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *b < 0 {
                        return Ok(Value::Float((*a as f64).powf(*b as f64)));
                    }
                    return Ok(Value::Int(a.pow(*b as u32)));
                }
                if l.as_float().is_some() && r.as_float().is_some() {
                    return Ok(Value::Float(
                        l.as_float().unwrap().powf(r.as_float().unwrap()),
                    ));
                }
                Err(self.err(line, "operator '**' requires numeric operands"))
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                self.stats.borrow_mut().comparisons += 1;
                self.compare_op(op, &l, &r, line)
            }
            BinOp::And | BinOp::Or => unreachable!(),
        }
    }

    fn compare_op(&self, op: BinOp, l: &Value, r: &Value, line: usize) -> Result<Value, RunError> {
        let ordering = match (l, r) {
            (Value::Int(a), Value::Int(b)) => Some(a.cmp(b)),
            _ if l.as_float().is_some() && r.as_float().is_some() => {
                let a = l.as_float().unwrap();
                let b = r.as_float().unwrap();
                a.partial_cmp(&b)
            }
            (Value::Str(a), Value::Str(b)) => Some(a.cmp(b)),
            _ => None,
        };
        match op {
            BinOp::Eq => Ok(Value::Bool(l.value_eq(r))),
            BinOp::Ne => Ok(Value::Bool(!l.value_eq(r))),
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                let ord = match ordering {
                    Some(o) => o,
                    None => {
                        return Err(self.err(line, format!(
                            "cannot compare {} and {}",
                            l.type_name(),
                            r.type_name()
                        )))
                    }
                };
                use std::cmp::Ordering::*;
                let res = match op {
                    BinOp::Lt => ord == Less,
                    BinOp::Gt => ord == Greater,
                    BinOp::Le => ord != Greater,
                    BinOp::Ge => ord != Less,
                    _ => unreachable!(),
                };
                Ok(Value::Bool(res))
            }
            _ => unreachable!(),
        }
    }

    fn eval_unop(&self, u: &UnOpExpr, env: EnvRef) -> Result<Value, RunError> {
        let v = self.eval_expr(&u.operand, env)?;
        match u.op {
            UnOp::Neg => match v {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(self.err(u.line, format!(
                    "unary '-' not supported for {}",
                    v.type_name()
                ))),
            },
            UnOp::Not => match v {
                Value::Bool(b) => Ok(Value::Bool(!b)),
                _ => Err(self.err(u.line, "operator 'not' requires OrionBool")),
            },
        }
    }

    fn eval_call(&self, c: &CallExpr, env: EnvRef) -> Result<Value, RunError> {
        if let Expr::Var(name) = &*c.callee {
            if let Some(v) = self.lookup_var_opt(name, &env) {
                if let Value::Function(f) = &v {
                    return self.call_function(f.clone(), c, env);
                }
            }
            return self.call_builtin(name, c, env);
        }
        let callee = self.eval_expr(&c.callee, env.clone())?;
        if let Value::Function(f) = callee {
            return self.call_function(f, c, env);
        }
        Err(self.err(c.line, "attempt to call non-function value"))
    }

    fn call_function(
        &self,
        f: Rc<FunctionValue>,
        c: &CallExpr,
        caller_env: EnvRef,
    ) -> Result<Value, RunError> {
        if c.args.len() != f.params.len() {
            return Err(self.err(
                c.line,
                format!(
                    "function '{}' expects {} arguments, got {}",
                    f.name,
                    f.params.len(),
                    c.args.len()
                ),
            ));
        }
        let mut args = Vec::new();
        for a in &c.args {
            args.push(self.eval_expr(a, caller_env.clone())?);
        }
        let depth = *self.depth.borrow();
        if depth >= MAX_DEPTH {
            return Err(self.err(
                c.line,
                format!("recursion depth exceeded (max {})", MAX_DEPTH),
            ));
        }
        *self.depth.borrow_mut() += 1;
        self.stats.borrow_mut().function_calls += 1;
        let result = (|| {
            let new_env = Rc::new(RefCell::new(Environment::with_parent(f.closure.clone())));
            for (param, arg) in f.params.iter().zip(args.iter()) {
                self.check_type(&param.type_name, arg, c.line)?;
                new_env.borrow_mut().define(param.name.clone(), arg.clone());
            }
            let body_result = self.exec_block(&f.body, new_env);
            match body_result {
                Ok(()) => {
                    if f.return_type == "OrionNone" {
                        Ok(Value::None)
                    } else {
                        Err(self.err(c.line, format!(
                            "function '{}' did not return a value",
                            f.name
                        )))
                    }
                }
                Err(e) => {
                    if let Some(v) = decode_value(&e.message) {
                        self.check_type(&f.return_type, &v, e.line)?;
                        Ok(v)
                    } else {
                        Err(e)
                    }
                }
            }
        })();
        *self.depth.borrow_mut() -= 1;
        result
    }

    fn call_builtin(&self, name: &str, c: &CallExpr, env: EnvRef) -> Result<Value, RunError> {
        let mut args = Vec::new();
        for a in &c.args {
            args.push(self.eval_expr(a, env.clone())?);
        }
        self.stats.borrow_mut().function_calls += 1;
        match name {
            "len" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "len expects 1 argument"));
                }
                let n = match &args[0] {
                    Value::Str(s) => s.chars().count(),
                    Value::List(l) => l.borrow().len(),
                    Value::Dict(d) => d.borrow().entries.len(),
                    Value::Set(s) => s.borrow().len(),
                    Value::Tuple(t) => t.len(),
                    Value::Heap(h) => h.borrow().len(),
                    Value::Range(r) => (r.end - r.start).max(0) as usize,
                    _ => {
                        return Err(self.err(c.line, "len not supported for this type"));
                    }
                };
                Ok(Value::Int(n as i64))
            }
            "range" => {
                if args.is_empty() || args.len() > 2 {
                    return Err(self.err(c.line, "range expects 1 or 2 arguments"));
                }
                let (start, end) = if args.len() == 1 {
                    (0, self.expect_int(&args[0], c.line)?)
                } else {
                    (
                        self.expect_int(&args[0], c.line)?,
                        self.expect_int(&args[1], c.line)?,
                    )
                };
                Ok(Value::Range(Rc::new(RangeValue { start, end })))
            }
            "min" | "max" => {
                if args.is_empty() {
                    return Err(self.err(c.line, format!("{} expects at least 1 argument", name)));
                }
                let mut best = args[0].clone();
                for v in args.iter().skip(1) {
                    self.stats.borrow_mut().comparisons += 1;
                    let cmp = self.partial_cmp(v, &best, c.line)?;
                    let use_new = if name == "min" {
                        std::cmp::Ordering::Less == cmp
                    } else {
                        std::cmp::Ordering::Greater == cmp
                    };
                    if use_new {
                        best = v.clone();
                    }
                }
                Ok(best)
            }
            "sum" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "sum expects 1 argument"));
                }
                let items = match &args[0] {
                    Value::List(l) => l.borrow().clone(),
                    Value::Range(r) => {
                        let mut out = Vec::new();
                        let mut i = r.start;
                        while i < r.end {
                            out.push(Value::Int(i));
                            i += 1;
                        }
                        out
                    }
                    _ => return Err(self.err(c.line, "sum expects OrionList or OrionRange")),
                };
                let mut acc = Value::Int(0);
                for v in items {
                    acc = self.binary_op(BinOp::Add, acc, v, c.line)?;
                }
                Ok(acc)
            }
            "abs" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "abs expects 1 argument"));
                }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(self.err(c.line, "abs expects a number")),
                }
            }
            "int" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "int expects 1 argument"));
                }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Str(s) => s
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| self.err(c.line, "cannot convert string to int")),
                    Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                    _ => Err(self.err(c.line, "int not supported for this type")),
                }
            }
            "str" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "str expects 1 argument"));
                }
                Ok(Value::Str(args[0].to_display_string()))
            }
            "bool" => {
                if args.len() != 1 {
                    return Err(self.err(c.line, "bool expects 1 argument"));
                }
                Ok(Value::Bool(args[0].is_truthy()))
            }
            "print" => {
                let s = args
                    .iter()
                    .map(|v| v.to_display_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                self.output.borrow_mut().push(s);
                Ok(Value::None)
            }
            "input" => {
                let mut pos = self.input_pos.borrow_mut();
                if *pos < self.input_lines.len() {
                    let line = self.input_lines[*pos].clone();
                    *pos += 1;
                    Ok(Value::Str(line))
                } else {
                    Ok(Value::None)
                }
            }
            "OrionHeap" => {
                if !args.is_empty() {
                    return Err(self.err(c.line, "OrionHeap() expects no arguments"));
                }
                Ok(Value::Heap(Rc::new(RefCell::new(Vec::new()))))
            }
            "OrionSet" => {
                if !args.is_empty() {
                    return Err(self.err(c.line, "OrionSet() expects no arguments"));
                }
                Ok(Value::Set(Rc::new(RefCell::new(Vec::new()))))
            }
            "OrionTuple" => Ok(Value::Tuple(Rc::new(args))),
            "OrionList" => Ok(Value::List(Rc::new(RefCell::new(args)))),
            "OrionDict" => Ok(Value::Dict(Rc::new(RefCell::new(DictData::new())))),
            _ => Err(self.err(c.line, format!("unknown function '{}'", name))),
        }
    }

    fn partial_cmp(
        &self,
        a: &Value,
        b: &Value,
        line: usize,
    ) -> Result<std::cmp::Ordering, RunError> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
            _ if a.as_float().is_some() && b.as_float().is_some() => {
                let x = a.as_float().unwrap();
                let y = b.as_float().unwrap();
                x.partial_cmp(&y)
                    .ok_or_else(|| self.err(line, "cannot compare NaN"))
            }
            (Value::Str(x), Value::Str(y)) => Ok(x.cmp(y)),
            _ => Err(self.err(line, format!(
                "cannot compare {} and {}",
                a.type_name(),
                b.type_name()
            ))),
        }
    }

    fn expect_int(&self, v: &Value, line: usize) -> Result<i64, RunError> {
        match v {
            Value::Int(n) => Ok(*n),
            _ => Err(self.err(line, "expected OrionInt")),
        }
    }

    fn index_get(&self, obj: &Value, key: &Value, line: usize) -> Result<Value, RunError> {
        match (obj, key) {
            (Value::List(l), Value::Int(i)) => {
                let borrow = l.borrow();
                let idx = self.normalize_index(*i, borrow.len(), line)?;
                Ok(borrow[idx].clone())
            }
            (Value::Str(s), Value::Int(i)) => {
                let chars: Vec<char> = s.chars().collect();
                let idx = self.normalize_index(*i, chars.len(), line)?;
                Ok(Value::Str(chars[idx].to_string()))
            }
            (Value::Dict(d), _) => d
                .borrow()
                .get(key)
                .cloned()
                .ok_or_else(|| self.err(line, "key not found in dict")),
            (Value::Tuple(t), Value::Int(i)) => {
                let idx = self.normalize_index(*i, t.len(), line)?;
                Ok(t[idx].clone())
            }
            _ => Err(self.err(line, format!(
                "indexing not supported for {}",
                obj.type_name()
            ))),
        }
    }

    fn index_set(
        &self,
        obj: &Value,
        key: &Value,
        val: Value,
        line: usize,
    ) -> Result<(), RunError> {
        match (obj, key) {
            (Value::List(l), Value::Int(i)) => {
                let mut borrow = l.borrow_mut();
                let idx = self.normalize_index(*i, borrow.len(), line)?;
                borrow[idx] = val;
                Ok(())
            }
            (Value::Dict(d), _) => {
                d.borrow_mut().set(key.clone(), val);
                Ok(())
            }
            _ => Err(self.err(line, format!(
                "index assignment not supported for {}",
                obj.type_name()
            ))),
        }
    }

    fn normalize_index(&self, i: i64, len: usize, line: usize) -> Result<usize, RunError> {
        let idx = if i < 0 { i + len as i64 } else { i };
        if idx < 0 || idx >= len as i64 {
            Err(self.err(line, format!("index {} out of range (len {})", i, len)))
        } else {
            Ok(idx as usize)
        }
    }

    fn lookup_var(&self, name: &str, env: &EnvRef, line: usize) -> Result<Value, RunError> {
        self.lookup_var_opt(name, env)
            .ok_or_else(|| self.err(line, format!("undefined variable '{}'", name)))
    }

    fn lookup_var_opt(&self, name: &str, env: &EnvRef) -> Option<Value> {
        let mut cur = Some(env.clone());
        while let Some(e) = cur {
            let borrow = e.borrow();
            if let Some(v) = borrow.get_local(name) {
                return Some(v);
            }
            cur = borrow.parent();
        }
        None
    }

    fn set_var(&self, name: &str, val: Value, env: &EnvRef) -> bool {
        let mut cur = Some(env.clone());
        while let Some(e) = cur {
            let parent = {
                let mut borrow = e.borrow_mut();
                if borrow.set_local(name, val.clone()) {
                    return true;
                }
                borrow.parent()
            };
            cur = parent;
        }
        false
    }

    fn check_type(&self, type_name: &str, v: &Value, line: usize) -> Result<(), RunError> {
        let actual = v.type_name();
        let ok = match type_name {
            "OrionInt" => matches!(v, Value::Int(_)),
            "OrionFloat" => matches!(v, Value::Float(_) | Value::Int(_)),
            "OrionStr" => matches!(v, Value::Str(_)),
            "OrionBool" => matches!(v, Value::Bool(_)),
            "OrionNone" => matches!(v, Value::None),
            "OrionList" => matches!(v, Value::List(_)),
            "OrionDict" => matches!(v, Value::Dict(_)),
            "OrionSet" => matches!(v, Value::Set(_)),
            "OrionTuple" => matches!(v, Value::Tuple(_)),
            "OrionHeap" => matches!(v, Value::Heap(_)),
            "OrionRange" => matches!(v, Value::Range(_)),
            "OrionFunction" => matches!(v, Value::Function(_)),
            _ => {
                return Err(self.err(line, format!("unknown type name '{}'", type_name)));
            }
        };
        if ok {
            Ok(())
        } else {
            Err(self.err(line, format!(
                "type mismatch: expected {}, got {}",
                type_name, actual
            )))
        }
    }

    fn update_peak(&self, env: &EnvRef) {
        let total = self.count_all_elements(env);
        self.bump_peak(total);
    }
}

fn encode_value(v: &Value) -> String {
    match v {
        Value::Int(n) => format!("i{}", n),
        Value::Float(f) => format!("f{}", f),
        Value::Str(s) => format!("s{}", s),
        Value::Bool(b) => format!("b{}", b),
        Value::None => "n".to_string(),
        _ => "n".to_string(),
    }
}

fn decode_value(s: &str) -> Option<Value> {
    if let Some(rest) = s.strip_prefix("__return__") {
        if rest == "n" {
            return Some(Value::None);
        }
        if let Some(num) = rest.strip_prefix('i') {
            return num.parse().ok().map(Value::Int);
        }
        if let Some(num) = rest.strip_prefix('f') {
            return num.parse().ok().map(Value::Float);
        }
        if let Some(st) = rest.strip_prefix('s') {
            return Some(Value::Str(st.to_string()));
        }
        if let Some(b) = rest.strip_prefix('b') {
            return b.parse().ok().map(Value::Bool);
        }
        Some(Value::None)
    } else {
        None
    }
}