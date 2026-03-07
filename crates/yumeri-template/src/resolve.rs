use std::collections::HashMap;

use yumeri_types::Color;
use yumeri_ui::callback::AnyCallback;
use yumeri_ui::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use yumeri_ui::event::EventKind;
use yumeri_ui::style::{Align, Dimension, Direction, Edges, Justify, Position, Style};
use yumeri_ui::template_provider::{BindingValueKind, TemplateBuildContext, TokenValueKind};
use yumeri_ui::transition::{TransitionDef, TransitionProperty};

use crate::binding::Bindings;
use crate::schema::*;
use crate::state::StateSet;
use crate::token::{resolve_color_token, resolve_float_token, TokenValue};

pub fn resolve_template(
    template: &Template,
    global_tokens: &HashMap<String, TokenValue>,
    mut ctx: TemplateBuildContext,
) -> Element {
    let needs_merge = !template.tokens.is_empty() || !ctx.token_overrides.is_empty();
    let merged;
    let tokens = if needs_merge {
        merged = merge_tokens(global_tokens, &template.tokens, &ctx.token_overrides);
        &merged
    } else {
        global_tokens
    };

    let bindings = build_bindings(&ctx);
    let states = build_states(&ctx);
    let mut event_handlers = std::mem::take(&mut ctx.event_handlers);

    resolve_node(&template.root, tokens, &bindings, &states, &mut event_handlers, true)
}

fn merge_tokens(
    global: &HashMap<String, TokenValue>,
    template_local: &HashMap<String, TokenValue>,
    overrides: &[(String, TokenValueKind)],
) -> HashMap<String, TokenValue> {
    let mut merged = global.clone();
    // Template-local tokens (lower priority than global)
    for (k, v) in template_local {
        merged.entry(k.clone()).or_insert_with(|| v.clone());
    }
    // Instance-level token overrides (highest priority)
    for (k, v) in overrides {
        let tv = match v {
            TokenValueKind::Float(f) => TokenValue::Float(*f),
            TokenValueKind::Color([r, g, b, a]) => TokenValue::Color(*r, *g, *b, *a),
        };
        merged.insert(k.clone(), tv);
    }
    merged
}


fn build_bindings(ctx: &TemplateBuildContext) -> Bindings {
    let mut bindings = Bindings::new();
    for (k, v) in &ctx.bindings {
        match v {
            BindingValueKind::String(s) => bindings.set_string(k.clone(), s.clone()),
            BindingValueKind::Bool(b) => bindings.set_bool(k.clone(), *b),
            BindingValueKind::Float(f) => bindings.set_float(k.clone(), *f),
        }
    }
    bindings
}

fn build_states(ctx: &TemplateBuildContext) -> StateSet {
    let mut states = StateSet::new();
    for s in &ctx.states {
        states.insert(s.clone());
    }
    states
}

fn resolve_node(
    node: &TemplateNode,
    tokens: &HashMap<String, TokenValue>,
    bindings: &Bindings,
    states: &StateSet,
    event_handlers: &mut Vec<(Option<String>, EventKind, AnyCallback)>,
    is_root: bool,
) -> Element {
    // Check visibility binding
    if let Some(vis) = &node.visible {
        match vis {
            crate::binding::ValueOrBinding::Literal(false) => {
                return empty_container();
            }
            crate::binding::ValueOrBinding::Binding(key) => {
                if !bindings.get_bool(key).unwrap_or(true) {
                    return empty_container();
                }
            }
            _ => {}
        }
    }

    let mut style = resolve_partial_style(&node.style, tokens);

    // Apply state overrides
    let matching_states = states.find_matching_states(&node.states);
    let mut text_color_override: Option<Color> = None;
    for state_key in &matching_states {
        if let Some(state_override) = node.states.get(*state_key) {
            apply_state_override(&mut style, state_override, tokens, &mut text_color_override);
        }
    }

    // Resolve transitions
    let transitions: Vec<TransitionDef> = node
        .transitions
        .iter()
        .map(|t| resolve_transition(t))
        .collect();
    style.transitions = transitions;

    // Resolve props
    let props = resolve_props(node.props.as_ref(), tokens, bindings, text_color_override);

    // Resolve children
    let children: Vec<Element> = node
        .children
        .iter()
        .map(|child| resolve_node(child, tokens, bindings, states, event_handlers, false))
        .collect();

    // Map widget kind to WidgetType
    let widget_type = match node.widget {
        WidgetKind::Container => WidgetType::Container,
        WidgetKind::Column => WidgetType::Column,
        WidgetKind::Row => WidgetType::Row,
        WidgetKind::Stack => WidgetType::Stack,
        WidgetKind::Text => WidgetType::Text,
        WidgetKind::Image => WidgetType::Image,
        WidgetKind::Rect => WidgetType::Rect,
        WidgetKind::RoundedRect => WidgetType::RoundedRect,
        WidgetKind::Circle => WidgetType::Circle,
        WidgetKind::Ellipse => WidgetType::Ellipse,
    };

    // Collect event handlers for this node
    let node_handlers: Vec<(EventKind, AnyCallback)> = {
        let mut handlers = Vec::new();
        let mut i = 0;
        while i < event_handlers.len() {
            let matches = match (&event_handlers[i].0, &node.id) {
                // Handlers without a target ID attach to the root node
                (None, _) if is_root => true,
                // Handlers with a target ID attach to matching node
                (Some(handler_id), Some(node_id)) => handler_id == node_id,
                _ => false,
            };
            if matches {
                let (_, kind, cb) = event_handlers.remove(i);
                handlers.push((kind, cb));
            } else {
                i += 1;
            }
        }
        handlers
    };

    Element {
        key: None,
        kind: ElementKind::Widget(WidgetElement {
            widget_type,
            style,
            props,
            children,
            event_handlers: node_handlers,
            focusable: false,
        }),
    }
}

fn resolve_partial_style(ps: &PartialStyle, tokens: &HashMap<String, TokenValue>) -> Style {
    let mut style = Style::default();

    if let Some(d) = &ps.direction {
        style.direction = match d {
            DirectionKind::Row => Direction::Row,
            DirectionKind::Column => Direction::Column,
        };
    }
    if let Some(w) = &ps.width { style.width = resolve_dimension(w, tokens); }
    if let Some(h) = &ps.height { style.height = resolve_dimension(h, tokens); }
    if let Some(w) = &ps.min_width { style.min_width = resolve_dimension(w, tokens); }
    if let Some(h) = &ps.min_height { style.min_height = resolve_dimension(h, tokens); }
    if let Some(w) = &ps.max_width { style.max_width = resolve_dimension(w, tokens); }
    if let Some(h) = &ps.max_height { style.max_height = resolve_dimension(h, tokens); }
    if let Some(p) = &ps.padding { style.padding = resolve_edges(p, tokens); }
    if let Some(m) = &ps.margin { style.margin = resolve_edges(m, tokens); }
    if let Some(g) = &ps.gap { style.gap = resolve_float_token(g, tokens).unwrap_or(0.0); }
    if let Some(v) = ps.flex_grow { style.flex_grow = v; }
    if let Some(v) = ps.flex_shrink { style.flex_shrink = v; }
    if let Some(fb) = &ps.flex_basis { style.flex_basis = resolve_dimension(fb, tokens); }
    if let Some(a) = &ps.align_items { style.align_items = Some(resolve_align(a)); }
    if let Some(a) = &ps.align_self { style.align_self = Some(resolve_align(a)); }
    if let Some(j) = &ps.justify_content { style.justify_content = Some(resolve_justify(j)); }
    if let Some(p) = &ps.position {
        style.position = match p {
            PositionKind::Relative => Position::Relative,
            PositionKind::Absolute => Position::Absolute,
        };
    }
    if let Some(i) = &ps.inset { style.inset = resolve_edges(i, tokens); }
    if let Some(bg) = &ps.background { style.background = resolve_color_token(bg, tokens); }
    if let Some(bc) = &ps.border_color { style.border_color = resolve_color_token(bc, tokens); }
    if let Some(bw) = ps.border_width { style.border_width = bw; }
    if let Some(cr) = &ps.corner_radius {
        style.corner_radius = resolve_float_token(cr, tokens).unwrap_or(0.0);
    }
    if let Some(o) = ps.opacity { style.opacity = o; }
    if let Some(v) = ps.visible { style.visible = v; }
    if let Some(t) = ps.translate { style.translate = t; }
    if let Some(s) = ps.scale { style.scale = s; }
    if let Some(r) = ps.rotation { style.rotation = r; }
    if let Some(to) = ps.transform_origin { style.transform_origin = to; }

    style
}

fn resolve_dimension(dv: &DimensionValue, tokens: &HashMap<String, TokenValue>) -> Dimension {
    match dv {
        DimensionValue::Auto => Dimension::Auto,
        DimensionValue::Px(vot) => {
            Dimension::Px(resolve_float_token(vot, tokens).unwrap_or(0.0))
        }
        DimensionValue::Percent(p) => Dimension::Percent(*p),
    }
}

fn resolve_edges(ev: &EdgesValue, tokens: &HashMap<String, TokenValue>) -> Edges {
    match ev {
        EdgesValue::All(v) => Edges::all(resolve_float_token(v, tokens).unwrap_or(0.0)),
        EdgesValue::Symmetric(h, v) => Edges::symmetric(
            resolve_float_token(h, tokens).unwrap_or(0.0),
            resolve_float_token(v, tokens).unwrap_or(0.0),
        ),
        EdgesValue::Each { top, right, bottom, left } => Edges {
            top: resolve_float_token(top, tokens).unwrap_or(0.0),
            right: resolve_float_token(right, tokens).unwrap_or(0.0),
            bottom: resolve_float_token(bottom, tokens).unwrap_or(0.0),
            left: resolve_float_token(left, tokens).unwrap_or(0.0),
        },
    }
}

fn resolve_align(a: &AlignKind) -> Align {
    match a {
        AlignKind::Start => Align::Start,
        AlignKind::End => Align::End,
        AlignKind::Center => Align::Center,
        AlignKind::Stretch => Align::Stretch,
    }
}

fn resolve_justify(j: &JustifyKind) -> Justify {
    match j {
        JustifyKind::Start => Justify::Start,
        JustifyKind::End => Justify::End,
        JustifyKind::Center => Justify::Center,
        JustifyKind::SpaceBetween => Justify::SpaceBetween,
        JustifyKind::SpaceAround => Justify::SpaceAround,
        JustifyKind::SpaceEvenly => Justify::SpaceEvenly,
    }
}

fn resolve_transition(spec: &TransitionSpec) -> TransitionDef {
    let property = match spec.property {
        TransitionPropertyKind::Opacity => TransitionProperty::Opacity,
        TransitionPropertyKind::BackgroundColor => TransitionProperty::BackgroundColor,
        TransitionPropertyKind::Width => TransitionProperty::Width,
        TransitionPropertyKind::Height => TransitionProperty::Height,
        TransitionPropertyKind::CornerRadius => TransitionProperty::CornerRadius,
        TransitionPropertyKind::Translate => TransitionProperty::Translate,
        TransitionPropertyKind::Scale => TransitionProperty::Scale,
        TransitionPropertyKind::Rotation => TransitionProperty::Rotation,
    };
    TransitionDef::new(property)
        .duration_ms(spec.duration_ms)
        .easing(spec.easing.to_easing())
}

fn resolve_props(
    props: Option<&NodeProps>,
    tokens: &HashMap<String, TokenValue>,
    bindings: &Bindings,
    text_color_override: Option<Color>,
) -> WidgetProps {
    let Some(props) = props else {
        return WidgetProps::default();
    };

    let text = props.text.as_ref().and_then(|t| match t {
        crate::binding::ValueOrBinding::Literal(s) => Some(s.clone()),
        crate::binding::ValueOrBinding::Binding(key) => {
            bindings.get_string(key).map(|s| s.to_string())
        }
    });

    let font_size = props
        .font_size
        .as_ref()
        .and_then(|fs| resolve_float_token(fs, tokens));

    let text_color = text_color_override.or_else(|| {
        props
            .text_color
            .as_ref()
            .and_then(|tc| resolve_color_token(tc, tokens))
    });

    WidgetProps {
        text,
        font_size,
        text_color,
        ..Default::default()
    }
}

fn apply_state_override(
    style: &mut Style,
    so: &StateOverride,
    tokens: &HashMap<String, TokenValue>,
    text_color_override: &mut Option<Color>,
) {
    if let Some(bg) = &so.background {
        style.background = resolve_color_token(bg, tokens);
    }
    if let Some(bc) = &so.border_color {
        style.border_color = resolve_color_token(bc, tokens);
    }
    if let Some(bw) = so.border_width {
        style.border_width = bw;
    }
    if let Some(cr) = &so.corner_radius {
        style.corner_radius = resolve_float_token(cr, tokens).unwrap_or(style.corner_radius);
    }
    if let Some(o) = so.opacity {
        style.opacity = o;
    }
    if let Some(tc) = &so.text_color {
        *text_color_override = resolve_color_token(tc, tokens);
    }
    if let Some(t) = so.translate {
        style.translate = t;
    }
    if let Some(s) = so.scale {
        style.scale = s;
    }
    if let Some(r) = so.rotation {
        style.rotation = r;
    }
}

fn empty_container() -> Element {
    Element {
        key: None,
        kind: ElementKind::Widget(WidgetElement {
            widget_type: WidgetType::Container,
            style: Style {
                visible: false,
                ..Default::default()
            },
            props: WidgetProps::default(),
            children: Vec::new(),
            event_handlers: Vec::new(),
            focusable: false,
        }),
    }
}
