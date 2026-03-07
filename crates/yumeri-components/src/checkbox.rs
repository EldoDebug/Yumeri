use yumeri_ui::prelude::*;

pub struct Checkbox {
    checked: bool,
    label: Option<String>,
}

impl Checkbox {
    pub fn new(checked: bool) -> Self {
        Self {
            checked,
            label: None,
        }
    }

    pub fn label(mut self, text: impl Into<String>) -> Self {
        self.label = Some(text.into());
        self
    }
}

impl Component for Checkbox {
    fn view(&self, ctx: &mut ViewCtx) -> Element {
        ctx.template("Checkbox")
            .bind_string("check_text", if self.checked { "\u{2713}" } else { "" })
            .bind_string("label", self.label.as_deref().unwrap_or(""))
            .bind_bool("has_label", self.label.is_some())
            .state_if(self.checked, "checked")
            .on_click(ctx.callback(|this: &mut Self, _| {
                this.checked = !this.checked;
            }))
            .build()
    }
}
