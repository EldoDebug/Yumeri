use yumeri_renderer::Color;
use yumeri_renderer::ShapeType;
use yumeri_renderer::ui::Scene;

use crate::element::WidgetType;
use crate::tree::{UiNodeId, UiTree};

pub(crate) fn sync_to_scene(tree: &mut UiTree, scene: &mut Scene) {
    let root = match tree.root {
        Some(r) => r,
        None => return,
    };

    // Compute taffy layout
    let available = taffy::prelude::Size {
        width: taffy::prelude::AvailableSpace::Definite(tree.viewport_size.0),
        height: taffy::prelude::AvailableSpace::Definite(tree.viewport_size.1),
    };

    if let Some(root_taffy) = tree.nodes.get(root).map(|n| n.taffy_node) {
        tree.taffy
            .compute_layout(root_taffy, available)
            .expect("taffy compute_layout");
    }

    // Sync nodes depth-first
    sync_node_recursive(tree, scene, root, 0.0, 0.0, 0);
}

fn sync_node_recursive(
    tree: &mut UiTree,
    scene: &mut Scene,
    node_id: UiNodeId,
    parent_x: f32,
    parent_y: f32,
    z_index: i32,
) {
    let (taffy_node, widget_type, style_clone, props_clone, children, scene_node_exists) = {
        let node = match tree.nodes.get(node_id) {
            Some(n) => n,
            None => return,
        };
        (
            node.taffy_node,
            node.widget_type,
            node.style.clone(),
            node.props.clone(),
            node.children.clone(),
            node.scene_node.is_some(),
        )
    };

    let layout = tree.taffy.layout(taffy_node).expect("taffy layout").clone();
    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    let w = layout.size.width;
    let h = layout.size.height;

    if !style_clone.visible || w <= 0.0 || h <= 0.0 {
        // Remove scene node if exists and not visible
        if let Some(scene_id) = tree.nodes.get(node_id).and_then(|n| n.scene_node) {
            scene.remove(scene_id);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = None;
            }
        }
        return;
    }

    let needs_scene_node = needs_visual(widget_type, &style_clone);

    if needs_scene_node {
        let shape_type = shape_type_for_widget(widget_type, &style_clone);

        let scene_id = if scene_node_exists {
            tree.nodes.get(node_id).unwrap().scene_node.unwrap()
        } else {
            let id = scene.add(shape_type);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = Some(id);
            }
            id
        };

        // taffy gives top-left position + full size
        // Scene uses center + half-extents
        let cx = abs_x + w / 2.0;
        let cy = abs_y + h / 2.0;
        scene.set_position(scene_id, [cx, cy]);
        scene.set_size(scene_id, [w / 2.0, h / 2.0]);

        // Apply visual properties
        let effective_opacity = style_clone.opacity;
        if let Some(bg) = style_clone.background {
            scene.set_color(
                scene_id,
                Color::rgba(bg.r, bg.g, bg.b, bg.a * effective_opacity),
            );
        } else if widget_type == WidgetType::Button {
            let default_bg = Color::rgb(0.25, 0.46, 0.85);
            scene.set_color(
                scene_id,
                Color::rgba(
                    default_bg.r,
                    default_bg.g,
                    default_bg.b,
                    effective_opacity,
                ),
            );
        } else {
            scene.set_color(
                scene_id,
                Color::rgba(0.0, 0.0, 0.0, 0.0),
            );
        }

        scene.set_corner_radius(scene_id, style_clone.corner_radius);
        scene.set_z_index(scene_id, z_index);

        if let Some(tex_id) = props_clone.texture_id {
            scene.set_texture(
                scene_id,
                Some(yumeri_renderer::Texture {
                    id: tex_id,
                    uv_rect: yumeri_renderer::UvRect::default(),
                }),
            );
        }
    } else if scene_node_exists {
        if let Some(scene_id) = tree.nodes.get(node_id).and_then(|n| n.scene_node) {
            scene.remove(scene_id);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = None;
            }
        }
    }

    // Handle text content
    if let Some(text) = &props_clone.text {
        if !text.is_empty()
            && matches!(
                widget_type,
                WidgetType::Text | WidgetType::Button | WidgetType::TextInput
            )
        {
            let text_color = props_clone
                .text_color
                .unwrap_or(Color::WHITE);
            let font_size = props_clone.font_size.unwrap_or(16.0);
            let _line_height = props_clone.line_height.unwrap_or(font_size * 1.25);

            // Text is rendered as a child scene node if needed
            // For now, text rendering requires UiContext (font + glyph cache)
            // which is handled in app.rs during the full sync
            // Store text info for later rendering
            let _ = (text_color, font_size);
        }
    }

    // Recurse children
    let scroll_offset = props_clone.scroll_offset.unwrap_or([0.0, 0.0]);
    let child_x = abs_x + scroll_offset[0];
    let child_y = abs_y + scroll_offset[1];

    for (i, child_id) in children.iter().enumerate() {
        sync_node_recursive(tree, scene, *child_id, child_x, child_y, z_index + 1 + i as i32);
    }
}

fn needs_visual(widget_type: WidgetType, style: &crate::style::Style) -> bool {
    match widget_type {
        WidgetType::Container => style.background.is_some() || style.border_width > 0.0,
        WidgetType::Column | WidgetType::Row => {
            style.background.is_some() || style.border_width > 0.0
        }
        WidgetType::Stack => style.background.is_some(),
        WidgetType::Text => false, // text is rendered via set_text
        WidgetType::Button => true,
        WidgetType::Image => true,
        WidgetType::TextInput => true,
        WidgetType::Checkbox => true,
        WidgetType::ScrollView => false,
    }
}

fn shape_type_for_widget(widget_type: WidgetType, style: &crate::style::Style) -> ShapeType {
    match widget_type {
        WidgetType::Image => ShapeType::Rect,
        _ => {
            if style.corner_radius > 0.0 {
                ShapeType::RoundedRect
            } else {
                ShapeType::Rect
            }
        }
    }
}

pub(crate) fn sync_text_nodes(
    tree: &mut UiTree,
    scene: &mut Scene,
    font: &mut yumeri_font::Font,
    glyph_cache: &mut yumeri_renderer::texture::glyph_cache::GlyphCache,
) {
    let node_ids: Vec<UiNodeId> = tree.nodes.keys().collect();
    for node_id in node_ids {
        let (widget_type, text, text_color, font_size, line_height, scene_node, taffy_node) = {
            let node = match tree.nodes.get(node_id) {
                Some(n) => n,
                None => continue,
            };
            (
                node.widget_type,
                node.props.text.clone(),
                node.props.text_color,
                node.props.font_size,
                node.props.line_height,
                node.scene_node,
                node.taffy_node,
            )
        };

        if !matches!(
            widget_type,
            WidgetType::Text | WidgetType::Button | WidgetType::TextInput
        ) {
            continue;
        }

        let text = match text {
            Some(t) if !t.is_empty() => t,
            _ => continue,
        };

        let layout = tree.taffy.layout(taffy_node).ok().cloned();
        let layout = match layout {
            Some(l) => l,
            None => continue,
        };

        let color = text_color.unwrap_or(Color::WHITE);
        let size = font_size.unwrap_or(16.0);
        let lh = line_height.unwrap_or(size * 1.25);

        let text_style = yumeri_renderer::TextStyle {
            font_size: size,
            line_height: lh,
            color,
            max_width: Some(layout.size.width),
            ..Default::default()
        };

        // Determine parent scene node for text
        let parent_scene_node = if widget_type == WidgetType::Text {
            // Text widget: create a scene node if needed
            match scene_node {
                Some(id) => id,
                None => {
                    let id = scene.add(ShapeType::None);
                    if let Some(node) = tree.nodes.get_mut(node_id) {
                        node.scene_node = Some(id);
                    }

                    // Position the text node
                    let abs_pos = compute_absolute_position(tree, node_id);
                    scene.set_position(id, [
                        abs_pos.0 + layout.size.width / 2.0,
                        abs_pos.1 + layout.size.height / 2.0,
                    ]);
                    scene.set_size(id, [layout.size.width / 2.0, layout.size.height / 2.0]);

                    id
                }
            }
        } else {
            // Button/TextInput: text goes inside existing scene node
            match scene_node {
                Some(id) => id,
                None => continue,
            }
        };

        scene.set_text(parent_scene_node, font, &text, &text_style, glyph_cache);
    }
}

fn compute_absolute_position(tree: &UiTree, node_id: UiNodeId) -> (f32, f32) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut current = Some(node_id);

    while let Some(id) = current {
        if let Some(node) = tree.nodes.get(id) {
            if let Ok(layout) = tree.taffy.layout(node.taffy_node) {
                x += layout.location.x;
                y += layout.location.y;
            }
            current = node.parent;
        } else {
            break;
        }
    }

    (x, y)
}
