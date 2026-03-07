use std::collections::HashMap;
use crate::token::TokenValue;

pub fn default_tokens() -> HashMap<String, TokenValue> {
    let mut t = HashMap::new();

    // === Palette: Dream Pink ===
    t.insert("pink-50".into(),  TokenValue::Color(1.0, 0.96, 0.99, 1.0));
    t.insert("pink-100".into(), TokenValue::Color(1.0, 0.93, 0.98, 1.0));
    t.insert("pink-200".into(), TokenValue::Color(0.99, 0.81, 0.90, 1.0));
    t.insert("pink-300".into(), TokenValue::Color(1.0, 0.71, 0.87, 1.0));
    t.insert("pink-400".into(), TokenValue::Color(1.0, 0.55, 0.78, 1.0));
    t.insert("pink-500".into(), TokenValue::Color(1.0, 0.40, 0.56, 1.0));

    // === Palette: Dream Cyan ===
    t.insert("cyan-50".into(),  TokenValue::Color(0.94, 1.0, 1.0, 1.0));
    t.insert("cyan-100".into(), TokenValue::Color(0.80, 1.0, 1.0, 1.0));
    t.insert("cyan-200".into(), TokenValue::Color(0.77, 0.91, 0.97, 1.0));
    t.insert("cyan-300".into(), TokenValue::Color(0.54, 1.0, 1.0, 1.0));
    t.insert("cyan-400".into(), TokenValue::Color(0.24, 0.89, 0.93, 1.0));
    t.insert("cyan-500".into(), TokenValue::Color(0.17, 0.62, 0.85, 1.0));

    // === Palette: Neutral ===
    t.insert("neutral-0".into(),   TokenValue::Color(1.0, 1.0, 1.0, 1.0));
    t.insert("neutral-50".into(),  TokenValue::Color(0.99, 1.0, 1.0, 1.0));
    t.insert("neutral-100".into(), TokenValue::Color(0.94, 0.95, 0.97, 1.0));
    t.insert("neutral-200".into(), TokenValue::Color(0.90, 0.92, 0.95, 1.0));
    t.insert("neutral-300".into(), TokenValue::Color(0.80, 0.84, 0.89, 1.0));
    t.insert("neutral-400".into(), TokenValue::Color(0.67, 0.71, 0.78, 1.0));
    t.insert("neutral-500".into(), TokenValue::Color(0.55, 0.58, 0.66, 1.0));
    t.insert("neutral-600".into(), TokenValue::Color(0.42, 0.45, 0.54, 1.0));
    t.insert("neutral-700".into(), TokenValue::Color(0.29, 0.32, 0.39, 1.0));
    t.insert("neutral-800".into(), TokenValue::Color(0.18, 0.20, 0.25, 1.0));
    t.insert("neutral-900".into(), TokenValue::Color(0.10, 0.11, 0.15, 1.0));

    // === Spacing ===
    t.insert("space-1".into(),  TokenValue::Float(4.0));
    t.insert("space-2".into(),  TokenValue::Float(8.0));
    t.insert("space-3".into(),  TokenValue::Float(12.0));
    t.insert("space-4".into(),  TokenValue::Float(16.0));
    t.insert("space-6".into(),  TokenValue::Float(24.0));
    t.insert("space-8".into(),  TokenValue::Float(32.0));
    t.insert("space-12".into(), TokenValue::Float(48.0));

    // === Border Radius ===
    t.insert("radius-xs".into(),   TokenValue::Float(6.0));
    t.insert("radius-sm".into(),   TokenValue::Float(12.0));
    t.insert("radius-md".into(),   TokenValue::Float(16.0));
    t.insert("radius-lg".into(),   TokenValue::Float(24.0));
    t.insert("radius-full".into(), TokenValue::Float(9999.0));

    // === Typography ===
    t.insert("text-xs".into(),   TokenValue::Float(12.0));
    t.insert("text-sm".into(),   TokenValue::Float(14.0));
    t.insert("text-base".into(), TokenValue::Float(16.0));
    t.insert("text-lg".into(),   TokenValue::Float(18.0));
    t.insert("text-xl".into(),   TokenValue::Float(20.0));
    t.insert("text-2xl".into(),  TokenValue::Float(24.0));

    // === Semantic: Colors ===
    t.insert("primary".into(),        TokenValue::Alias("cyan-500".into()));
    t.insert("primary-hover".into(),  TokenValue::Alias("cyan-400".into()));
    t.insert("primary-light".into(),  TokenValue::Alias("cyan-100".into()));
    t.insert("secondary".into(),      TokenValue::Alias("pink-500".into()));
    t.insert("secondary-hover".into(),TokenValue::Alias("pink-400".into()));
    t.insert("secondary-light".into(),TokenValue::Alias("pink-100".into()));

    t.insert("surface".into(),         TokenValue::Alias("neutral-0".into()));
    t.insert("surface-hover".into(),   TokenValue::Alias("neutral-100".into()));
    t.insert("surface-variant".into(), TokenValue::Alias("neutral-50".into()));

    t.insert("on-primary".into(),         TokenValue::Alias("neutral-0".into()));
    t.insert("on-secondary".into(),       TokenValue::Alias("neutral-0".into()));
    t.insert("on-surface".into(),         TokenValue::Alias("neutral-700".into()));
    t.insert("on-surface-variant".into(), TokenValue::Alias("neutral-500".into()));

    t.insert("border".into(),        TokenValue::Alias("neutral-200".into()));
    t.insert("border-strong".into(), TokenValue::Alias("neutral-300".into()));

    t.insert("text".into(),          TokenValue::Alias("neutral-700".into()));
    t.insert("text-heading".into(),  TokenValue::Alias("neutral-900".into()));
    t.insert("text-secondary".into(),TokenValue::Alias("neutral-500".into()));
    t.insert("text-disabled".into(), TokenValue::Alias("neutral-400".into()));

    t.insert("error".into(),       TokenValue::Alias("pink-500".into()));
    t.insert("error-light".into(), TokenValue::Alias("pink-50".into()));

    t.insert("success-color".into(), TokenValue::Color(0.36, 0.93, 0.79, 1.0));
    t.insert("warning-color".into(), TokenValue::Color(1.0, 0.84, 0.40, 1.0));
    t.insert("info-color".into(),    TokenValue::Color(0.42, 0.72, 1.0, 1.0));

    t
}
