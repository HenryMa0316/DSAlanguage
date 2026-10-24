use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::{Block, Param};
use crate::env::Environment;

pub type EnvRef = Rc<RefCell<Environment>>;

#[derive(Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<DictData>>),
    Set(Rc<RefCell<Vec<Value>>>),
    Tuple(Rc<Vec<Value>>),
    Function(Rc<FunctionValue>),
    Range(Rc<RangeValue>),
    Heap(Rc<RefCell<Vec<Value>>>),
}

pub struct FunctionValue {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: String,
    pub body: Block,
    pub closure: EnvRef,
}

pub struct RangeValue {
    pub start: i64,
    pub end: i64,
}

pub struct DictData {
    pub entries: Vec<(Value, Value)>,
}

impl DictData {
    pub fn new() -> Self {
        DictData { entries: Vec::new() }
    }

    pub fn get(&self, key: &Value) -> Option<&Value> {
        self.entries.iter().find(|(k, _)| k.value_eq(key)).map(|(_, v)| v)
    }

    pub fn set(&mut self, key: Value, val: Value) {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k.value_eq(&key)) {
            self.entries[pos].1 = val;
        } else {
            self.entries.push((key, val));
        }
    }

    pub fn remove(&mut self, key: &Value) -> Option<Value> {
        if let Some(pos) = self.entries.iter().position(|(k, _)| k.value_eq(key)) {
            let (_, v) = self.entries.remove(pos);
            Some(v)
        } else {
            None
        }
    }

    pub fn keys(&self) -> Vec<Value> {
        self.entries.iter().map(|(k, _)| k.clone()).collect()
    }

    pub fn values(&self) -> Vec<Value> {
        self.entries.iter().map(|(_, v)| v.clone()).collect()
    }

    pub fn items(&self) -> Vec<Value> {
        self.entries
            .iter()
            .map(|(k, v)| Value::Tuple(Rc::new(vec![k.clone(), v.clone()])))
            .collect()
    }
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "OrionInt",
            Value::Float(_) => "OrionFloat",
            Value::Str(_) => "OrionStr",
            Value::Bool(_) => "OrionBool",
            Value::None => "OrionNone",
            Value::List(_) => "OrionList",
            Value::Dict(_) => "OrionDict",
            Value::Set(_) => "OrionSet",
            Value::Tuple(_) => "OrionTuple",
            Value::Function(_) => "OrionFunction",
            Value::Range(_) => "OrionRange",
            Value::Heap(_) => "OrionHeap",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::None => false,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(v) => !v.borrow().is_empty(),
            Value::Dict(d) => !d.borrow().entries.is_empty(),
            Value::Set(s) => !s.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Heap(h) => !h.borrow().is_empty(),
            Value::Range(r) => r.start < r.end,
            Value::Function(_) => true,
        }
    }

    pub fn value_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::None, Value::None) => true,
            (Value::List(a), Value::List(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.value_eq(y))
            }
            (Value::Tuple(a), Value::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.value_eq(y))
            }
            _ => false,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        if let Value::Int(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::Str(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn to_display_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Str(s) => s.clone(),
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::None => "none".to_string(),
            Value::List(v) => {
                let items: Vec<String> = v.borrow().iter().map(|x| x.to_repr_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Dict(d) => {
                let items: Vec<String> = d
                    .borrow()
                    .entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k.to_repr_string(), v.to_repr_string()))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Set(s) => {
                let items: Vec<String> = s.borrow().iter().map(|x| x.to_repr_string()).collect();
                format!("{{{}}}", items.join(", "))
            }
            Value::Tuple(t) => {
                let items: Vec<String> = t.iter().map(|x| x.to_repr_string()).collect();
                format!("({})", items.join(", "))
            }
            Value::Function(f) => format!("<function {}>", f.name),
            Value::Range(r) => format!("range({}, {})", r.start, r.end),
            Value::Heap(h) => {
                let items: Vec<String> = h.borrow().iter().map(|x| x.to_repr_string()).collect();
                format!("Heap[{}]", items.join(", "))
            }
        }
    }

    pub fn to_repr_string(&self) -> String {
        match self {
            Value::Str(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            _ => self.to_display_string(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}