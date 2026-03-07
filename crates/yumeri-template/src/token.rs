use std::collections::HashMap;
use serde::Deserialize;
use yumeri_types::Color;

#[derive(Clone, Debug, Deserialize)]
pub enum TokenValue {
    Color(f32, f32, f32, f32),
    Float(f32),
    Alias(String),
}

impl TokenValue {
    pub fn resolve_color(
        &self,
        all_tokens: &HashMap<String, TokenValue>,
        depth: usize,
    ) -> Option<Color> {
        if depth > 32 {
            return None; // circular reference guard
        }
        match self {
            TokenValue::Color(r, g, b, a) => Some(Color::rgba(*r, *g, *b, *a)),
            TokenValue::Alias(key) => all_tokens
                .get(key)
                .and_then(|v| v.resolve_color(all_tokens, depth + 1)),
            TokenValue::Float(_) => None,
        }
    }

    pub fn resolve_float(
        &self,
        all_tokens: &HashMap<String, TokenValue>,
        depth: usize,
    ) -> Option<f32> {
        if depth > 32 {
            return None;
        }
        match self {
            TokenValue::Float(v) => Some(*v),
            TokenValue::Alias(key) => all_tokens
                .get(key)
                .and_then(|v| v.resolve_float(all_tokens, depth + 1)),
            TokenValue::Color(_, _, _, _) => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub enum ValueOrToken<T: Clone> {
    Token(String),
    Literal(T),
}

impl<T: Clone> ValueOrToken<T> {
    pub fn is_token(&self) -> bool {
        matches!(self, ValueOrToken::Token(_))
    }
}

pub fn resolve_color_token(
    vot: &ValueOrToken<(f32, f32, f32, f32)>,
    tokens: &HashMap<String, TokenValue>,
) -> Option<Color> {
    match vot {
        ValueOrToken::Literal((r, g, b, a)) => Some(Color::rgba(*r, *g, *b, *a)),
        ValueOrToken::Token(key) => tokens
            .get(key)
            .and_then(|v| v.resolve_color(tokens, 0)),
    }
}

pub fn resolve_float_token(
    vot: &ValueOrToken<f32>,
    tokens: &HashMap<String, TokenValue>,
) -> Option<f32> {
    match vot {
        ValueOrToken::Literal(v) => Some(*v),
        ValueOrToken::Token(key) => tokens
            .get(key)
            .and_then(|v| v.resolve_float(tokens, 0)),
    }
}

pub fn validate_tokens(tokens: &HashMap<String, TokenValue>) -> Result<(), String> {
    for (key, value) in tokens {
        if let TokenValue::Alias(_) = value {
            resolve_alias(key, tokens, &mut Vec::new())?;
        }
    }
    Ok(())
}

fn resolve_alias(
    key: &str,
    tokens: &HashMap<String, TokenValue>,
    visited: &mut Vec<String>,
) -> Result<(), String> {
    if visited.contains(&key.to_string()) {
        return Err(format!("circular alias detected: {} -> {}", visited.join(" -> "), key));
    }
    visited.push(key.to_string());
    match tokens.get(key) {
        Some(TokenValue::Alias(target)) => resolve_alias(target, tokens, visited),
        Some(_) => Ok(()),
        None => Err(format!("undefined token reference: '{}'", key)),
    }
}
