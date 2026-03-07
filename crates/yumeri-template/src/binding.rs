use std::collections::HashMap;
use serde::Deserialize;
use yumeri_types::Color;

#[derive(Clone, Debug)]
pub enum BindingValue {
    String(String),
    Bool(bool),
    Float(f32),
    Color(Color),
}

#[derive(Clone, Debug, Default)]
pub struct Bindings(pub HashMap<String, BindingValue>);

impl Bindings {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn set_string(&mut self, key: impl Into<String>, val: impl Into<String>) {
        self.0.insert(key.into(), BindingValue::String(val.into()));
    }

    pub fn set_bool(&mut self, key: impl Into<String>, val: bool) {
        self.0.insert(key.into(), BindingValue::Bool(val));
    }

    pub fn set_float(&mut self, key: impl Into<String>, val: f32) {
        self.0.insert(key.into(), BindingValue::Float(val));
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.0.get(key) {
            Some(BindingValue::String(s)) => Some(s),
            _ => None,
        }
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.0.get(key) {
            Some(BindingValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    pub fn get_float(&self, key: &str) -> Option<f32> {
        match self.0.get(key) {
            Some(BindingValue::Float(f)) => Some(*f),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum ValueOrBinding<T: Clone> {
    Binding(String),
    Literal(T),
}
