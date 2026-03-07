use crate::callback::AnyCallback;
use crate::element::Element;
use crate::event::EventKind;
use crate::template_provider::{
    BindingValueKind, TemplateBuildContext, TemplateProvider, TokenValueKind,
};
use crate::widget::Container;

pub struct TemplateViewBuilder {
    name: String,
    provider: Option<*const dyn TemplateProvider>,
    ctx: TemplateBuildContext,
}

impl TemplateViewBuilder {
    pub(crate) fn new(name: &str, provider: Option<*const dyn TemplateProvider>) -> Self {
        Self {
            name: name.to_string(),
            provider,
            ctx: TemplateBuildContext {
                bindings: Vec::new(),
                states: Vec::new(),
                event_handlers: Vec::new(),
                token_overrides: Vec::new(),
            },
        }
    }

    pub fn bind_string(&mut self, key: &str, val: &str) -> &mut Self {
        self.ctx
            .bindings
            .push((key.to_string(), BindingValueKind::String(val.to_string())));
        self
    }

    pub fn bind_bool(&mut self, key: &str, val: bool) -> &mut Self {
        self.ctx
            .bindings
            .push((key.to_string(), BindingValueKind::Bool(val)));
        self
    }

    pub fn bind_float(&mut self, key: &str, val: f32) -> &mut Self {
        self.ctx
            .bindings
            .push((key.to_string(), BindingValueKind::Float(val)));
        self
    }

    pub fn state(&mut self, name: &str) -> &mut Self {
        self.ctx.states.push(name.to_string());
        self
    }

    pub fn state_if(&mut self, cond: bool, name: &str) -> &mut Self {
        if cond {
            self.ctx.states.push(name.to_string());
        }
        self
    }

    pub fn on_click(&mut self, cb: AnyCallback) -> &mut Self {
        self.ctx
            .event_handlers
            .push((None, EventKind::Click, cb));
        self
    }

    pub fn on_click_at(&mut self, id: &str, cb: AnyCallback) -> &mut Self {
        self.ctx
            .event_handlers
            .push((Some(id.to_string()), EventKind::Click, cb));
        self
    }

    pub fn token_float(&mut self, key: &str, val: f32) -> &mut Self {
        self.ctx
            .token_overrides
            .push((key.to_string(), TokenValueKind::Float(val)));
        self
    }

    pub fn token_color(&mut self, key: &str, val: [f32; 4]) -> &mut Self {
        self.ctx
            .token_overrides
            .push((key.to_string(), TokenValueKind::Color(val)));
        self
    }

    pub fn build(&mut self) -> Element {
        if let Some(provider) = self.provider {
            // SAFETY: The provider pointer originates from UiTree::template_provider (a Box<dyn TemplateProvider>).
            // ViewCtx (and thus TemplateViewBuilder) is created and consumed synchronously within
            // Component::view(), which borrows UiTree. The provider outlives this entire call.
            let provider = unsafe { &*provider };
            let ctx = std::mem::replace(
                &mut self.ctx,
                TemplateBuildContext {
                    bindings: Vec::new(),
                    states: Vec::new(),
                    event_handlers: Vec::new(),
                    token_overrides: Vec::new(),
                },
            );
            if let Some(element) = provider.build_template(&self.name, ctx) {
                return element;
            }
        }
        // Fallback: empty container
        Container::new().into()
    }
}
