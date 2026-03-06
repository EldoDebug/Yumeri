use taffy::prelude::*;

use crate::style;

pub(crate) fn to_taffy_style(s: &style::Style) -> taffy::Style {
    taffy::Style {
        display: Display::Flex,
        flex_direction: match s.direction {
            style::Direction::Row => FlexDirection::Row,
            style::Direction::Column => FlexDirection::Column,
        },
        size: Size {
            width: to_dimension(s.width),
            height: to_dimension(s.height),
        },
        min_size: Size {
            width: to_dimension(s.min_width),
            height: to_dimension(s.min_height),
        },
        max_size: Size {
            width: to_dimension(s.max_width),
            height: to_dimension(s.max_height),
        },
        padding: to_rect_length(s.padding),
        margin: to_rect_length_auto(s.margin),
        gap: Size {
            width: length(s.gap),
            height: length(s.gap),
        },
        flex_grow: s.flex_grow,
        flex_shrink: s.flex_shrink,
        flex_basis: to_dimension(s.flex_basis),
        align_items: s.align_items.map(to_align_items),
        align_self: s.align_self.map(to_align_self),
        justify_content: s.justify_content.map(to_justify_content),
        position: match s.position {
            style::Position::Relative => taffy::Position::Relative,
            style::Position::Absolute => taffy::Position::Absolute,
        },
        inset: taffy::Rect {
            left: to_length_percent_auto(s.inset.left),
            right: to_length_percent_auto(s.inset.right),
            top: to_length_percent_auto(s.inset.top),
            bottom: to_length_percent_auto(s.inset.bottom),
        },
        ..Default::default()
    }
}

fn to_dimension(d: style::Dimension) -> Dimension {
    match d {
        style::Dimension::Auto => Dimension::Auto,
        style::Dimension::Px(v) => Dimension::from_length(v),
        style::Dimension::Percent(v) => Dimension::from_percent(v),
    }
}

fn to_length_percent_auto(v: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::Length(v)
}

fn to_rect_length(edges: style::Edges) -> taffy::Rect<LengthPercentage> {
    taffy::Rect {
        left: length(edges.left),
        right: length(edges.right),
        top: length(edges.top),
        bottom: length(edges.bottom),
    }
}

fn to_rect_length_auto(edges: style::Edges) -> taffy::Rect<LengthPercentageAuto> {
    taffy::Rect {
        left: LengthPercentageAuto::Length(edges.left),
        right: LengthPercentageAuto::Length(edges.right),
        top: LengthPercentageAuto::Length(edges.top),
        bottom: LengthPercentageAuto::Length(edges.bottom),
    }
}

fn to_align_items(a: style::Align) -> AlignItems {
    match a {
        style::Align::Start => AlignItems::FlexStart,
        style::Align::End => AlignItems::FlexEnd,
        style::Align::Center => AlignItems::Center,
        style::Align::Stretch => AlignItems::Stretch,
    }
}

fn to_align_self(a: style::Align) -> AlignSelf {
    match a {
        style::Align::Start => AlignSelf::FlexStart,
        style::Align::End => AlignSelf::FlexEnd,
        style::Align::Center => AlignSelf::Center,
        style::Align::Stretch => AlignSelf::Stretch,
    }
}

fn to_justify_content(j: style::Justify) -> JustifyContent {
    match j {
        style::Justify::Start => JustifyContent::FlexStart,
        style::Justify::End => JustifyContent::FlexEnd,
        style::Justify::Center => JustifyContent::Center,
        style::Justify::SpaceBetween => JustifyContent::SpaceBetween,
        style::Justify::SpaceAround => JustifyContent::SpaceAround,
        style::Justify::SpaceEvenly => JustifyContent::SpaceEvenly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_style_converts() {
        let s = style::Style::default();
        let ts = to_taffy_style(&s);
        assert_eq!(ts.flex_direction, FlexDirection::Column);
        assert_eq!(ts.flex_grow, 0.0);
        assert_eq!(ts.flex_shrink, 1.0);
    }

    #[test]
    fn dimension_converts() {
        assert!(matches!(to_dimension(style::Dimension::Auto), Dimension::Auto));
        assert!(matches!(
            to_dimension(style::Dimension::Px(100.0)),
            Dimension::Length(v) if (v - 100.0).abs() < f32::EPSILON
        ));
        assert!(matches!(
            to_dimension(style::Dimension::Percent(0.5)),
            Dimension::Percent(v) if (v - 0.5).abs() < f32::EPSILON
        ));
    }

    #[test]
    fn row_direction() {
        let s = style::Style {
            direction: style::Direction::Row,
            ..Default::default()
        };
        let ts = to_taffy_style(&s);
        assert_eq!(ts.flex_direction, FlexDirection::Row);
    }
}
