use std::collections::HashMap;

use crate::default_tokens::default_tokens;
use crate::registry::TemplateRegistry;
use crate::state::StateSet;
use crate::token::{validate_tokens, TokenValue};

// --- Token tests ---

#[test]
fn token_alias_resolves_color() {
    let tokens = default_tokens();
    let primary = tokens.get("primary").unwrap();
    let color = primary.resolve_color(&tokens, 0).unwrap();
    // "primary" -> Alias("cyan-500") -> Color(0.17, 0.62, 0.85, 1.0)
    assert!((color.r - 0.17).abs() < 0.01);
    assert!((color.g - 0.62).abs() < 0.01);
    assert!((color.b - 0.85).abs() < 0.01);
}

#[test]
fn token_alias_resolves_float() {
    let tokens = default_tokens();
    let space = tokens.get("space-2").unwrap();
    let val = space.resolve_float(&tokens, 0).unwrap();
    assert!((val - 8.0).abs() < 0.01);
}

#[test]
fn token_circular_alias_detected() {
    let mut tokens = HashMap::new();
    tokens.insert("a".into(), TokenValue::Alias("b".into()));
    tokens.insert("b".into(), TokenValue::Alias("a".into()));
    let result = validate_tokens(&tokens);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("circular"));
}

#[test]
fn token_dangling_alias_detected() {
    let mut tokens = HashMap::new();
    tokens.insert("a".into(), TokenValue::Alias("nonexistent".into()));
    let result = validate_tokens(&tokens);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("undefined token reference"));
}

#[test]
fn validate_default_tokens_ok() {
    let tokens = default_tokens();
    assert!(validate_tokens(&tokens).is_ok());
}

// --- State matching tests ---

#[test]
fn state_matching_most_specific_wins() {
    let mut states = StateSet::new();
    states.insert("checked");
    states.insert("hovered");

    let mut map = HashMap::new();
    map.insert("default".to_string(), ());
    map.insert("checked".to_string(), ());
    map.insert("hovered".to_string(), ());
    map.insert("checked+hovered".to_string(), ());

    let matches = states.find_matching_states(&map);
    // Most specific first: "checked+hovered" (2 parts), then "checked" and "hovered" (1 part each), then "default" (0)
    assert_eq!(matches[0], "checked+hovered");
    assert_eq!(matches.len(), 4);
    assert_eq!(*matches.last().unwrap(), "default");
}

#[test]
fn state_matching_no_match() {
    let mut states = StateSet::new();
    states.insert("disabled");

    let mut map = HashMap::new();
    map.insert("default".to_string(), ());
    map.insert("checked".to_string(), ());

    let matches = states.find_matching_states(&map);
    // Only "default" should match
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0], "default");
}

// --- Template parsing tests ---

#[test]
fn parse_simple_template() {
    let ron_str = r#"
Template(
    name: "TestWidget",
    tokens: {
        "my_size": Float(42.0),
    },
    animations: {},
    root: (
        widget: Container,
        style: (
            width: Px(Literal(100.0)),
            height: Px(Token("my_size")),
            background: Token("primary"),
        ),
        children: [],
    ),
)
"#;
    let mut registry = TemplateRegistry::new();
    registry.load_str(ron_str).unwrap();
    let template = registry.get("TestWidget").unwrap();
    assert_eq!(template.name, "TestWidget");
}

#[test]
fn parse_checkbox_template() {
    let ron_str = include_str!("../../yumeri-components/templates/checkbox.template.ron");
    let mut registry = TemplateRegistry::new();
    registry.load_str(ron_str).unwrap();
    let template = registry.get("Checkbox").unwrap();
    assert_eq!(template.name, "Checkbox");
}

#[test]
fn resolve_simple_template() {
    use yumeri_ui::element::ElementKind;
    use yumeri_ui::template_provider::TemplateBuildContext;

    let ron_str = r#"
Template(
    name: "Test",
    tokens: {},
    animations: {},
    root: (
        widget: Row,
        style: (gap: Literal(10.0)),
        children: [
            (
                widget: Text,
                props: (
                    text: Binding("label"),
                    font_size: Literal(16.0),
                ),
            ),
        ],
    ),
)
"#;
    let mut registry = TemplateRegistry::new();
    registry.load_str(ron_str).unwrap();

    let ctx = TemplateBuildContext {
        bindings: vec![("label".into(), yumeri_ui::template_provider::BindingValueKind::String("Hello".into()))],
        states: vec![],
        event_handlers: vec![],
        token_overrides: vec![],
    };

    let element = registry.build_template("Test", ctx).unwrap();
    match &element.kind {
        ElementKind::Widget(w) => {
            assert_eq!(w.widget_type, yumeri_ui::element::WidgetType::Row);
            assert_eq!(w.children.len(), 1);
            match &w.children[0].kind {
                ElementKind::Widget(child) => {
                    assert_eq!(child.widget_type, yumeri_ui::element::WidgetType::Text);
                    assert_eq!(child.props.text.as_deref(), Some("Hello"));
                    assert_eq!(child.props.font_size, Some(16.0));
                }
                _ => panic!("expected widget child"),
            }
        }
        _ => panic!("expected widget root"),
    }
}

// Need to import TemplateProvider for build_template
use yumeri_ui::template_provider::TemplateProvider;
