use crate::element::Element;
use crate::callback::AnyCallback;
use crate::event::EventKind;

pub trait TemplateProvider {
    fn build_template(
        &self,
        name: &str,
        ctx: TemplateBuildContext,
    ) -> Option<Element>;
}

pub struct TemplateBuildContext {
    pub bindings: Vec<(String, BindingValueKind)>,
    pub states: Vec<String>,
    pub event_handlers: Vec<(Option<String>, EventKind, AnyCallback)>,
    pub token_overrides: Vec<(String, TokenValueKind)>,
}

pub enum BindingValueKind {
    String(String),
    Bool(bool),
    Float(f32),
}

pub enum TokenValueKind {
    Float(f32),
    Color([f32; 4]),
}
