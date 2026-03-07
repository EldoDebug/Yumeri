use yumeri_types::Color;
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
        let box_bg = if self.checked {
            Color::rgb(0.25, 0.46, 0.85)
        } else {
            Color::rgb(0.2, 0.2, 0.24)
        };

        let check_mark = if self.checked { "\u{2713}" } else { "" };

        let checkbox_box = Container::new()
            .width(Dimension::Px(20.0))
            .height(Dimension::Px(20.0))
            .background(box_bg)
            .corner_radius(4.0)
            .align_items(Align::Center)
            .justify_content(Justify::Center)
            .child(Text::new(check_mark).font_size(14.0).color(Color::WHITE));

        let mut row = Row::new()
            .gap(8.0)
            .align_items(Align::Center)
            .on_click(ctx.callback(|this: &mut Self, _| {
                this.checked = !this.checked;
            }))
            .child(checkbox_box);

        if let Some(ref label) = self.label {
            row = row.child(Text::new(label.clone()).font_size(16.0).color(Color::WHITE));
        }

        row.into()
    }
}
