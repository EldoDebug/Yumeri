use slotmap::SlotMap;
use yumeri_types::{Color, ShapeType};
use yumeri_renderer::ui::Scene;

use crate::element::WidgetType;
use crate::tree::{UiNode, UiNodeId, UiTree};

const DEFAULT_FONT_SIZE: f32 = 16.0;
const DEFAULT_LINE_HEIGHT_FACTOR: f32 = 1.25;
const DEFAULT_BUTTON_BG: Color = Color { r: 0.25, g: 0.46, b: 0.85, a: 1.0 };

pub(crate) fn sync_to_scene(
    tree: &mut UiTree,
    scene: &mut Scene,
    mut font: Option<&mut yumeri_font::Font>,
    glyph_cache: Option<&mut yumeri_renderer::texture::glyph_cache::GlyphCache>,
) {
    // Remove scene nodes orphaned by reconciliation
    for scene_id in tree.pending_scene_removals.drain(..) {
        scene.remove(scene_id);
    }

    let root = match tree.root {
        Some(r) => r,
        None => return,
    };

    let available = taffy::prelude::Size {
        width: taffy::prelude::AvailableSpace::Definite(tree.viewport_size.0),
        height: taffy::prelude::AvailableSpace::Definite(tree.viewport_size.1),
    };

    let root_taffy = match tree.nodes.get(root).map(|n| n.taffy_node) {
        Some(t) => t,
        None => return,
    };

    // Compute taffy layout with text measurement (split borrows: nodes + taffy)
    {
        let nodes = &tree.nodes;
        let taffy = &mut tree.taffy;
        if let Some(ref mut f) = font {
            taffy
                .compute_layout_with_measure(
                    root_taffy,
                    available,
                    |known_dims, avail_space, _node_id, node_ctx, _style| {
                        measure_text_node(nodes, &mut **f, known_dims, avail_space, node_ctx)
                    },
                )
                .expect("taffy compute_layout");
        } else {
            taffy
                .compute_layout(root_taffy, available)
                .expect("taffy compute_layout");
        }
    }

    // Sync nodes depth-first (text rendering happens inline if font/gc available)
    let mut text_ctx = match (font, glyph_cache) {
        (Some(f), Some(gc)) => Some(TextRenderCtx { font: f, glyph_cache: gc }),
        _ => None,
    };
    sync_node_recursive(tree, scene, &mut text_ctx, root, 0.0, 0.0, 0);
}

fn measure_text_node(
    nodes: &SlotMap<UiNodeId, UiNode>,
    font: &mut yumeri_font::Font,
    known_dimensions: taffy::prelude::Size<Option<f32>>,
    available_space: taffy::prelude::Size<taffy::prelude::AvailableSpace>,
    node_context: Option<&mut UiNodeId>,
) -> taffy::prelude::Size<f32> {
    let zero = taffy::prelude::Size { width: 0.0, height: 0.0 };

    let ui_node_id = match node_context {
        Some(id) => *id,
        None => return zero,
    };

    let node = match nodes.get(ui_node_id) {
        Some(n) => n,
        None => return zero,
    };

    if !node.widget_type.is_text_bearing() {
        return zero;
    }

    let text = match &node.props.text {
        Some(t) if !t.is_empty() => t.as_str(),
        _ => return zero,
    };

    let (font_size, line_height) = resolve_text_metrics(node.props.font_size, node.props.line_height);

    let max_width = known_dimensions.width.or(match available_space.width {
        taffy::prelude::AvailableSpace::Definite(w) => Some(w),
        _ => None,
    });

    let metrics = yumeri_font::TextMetrics::new(font_size, line_height);
    let mut buffer = yumeri_font::TextBuffer::new(font, metrics);
    if let Some(max_w) = max_width {
        buffer.set_size(font, Some(max_w), None);
    }
    buffer.set_text(font, text, &yumeri_font::FontAttrs::new());

    let glyphs = buffer.shape_and_layout(font);
    let text_width = glyphs.iter().map(|g| g.x + g.width).fold(0.0f32, f32::max);
    let text_height = buffer.layout_height();

    taffy::prelude::Size {
        width: known_dimensions.width.unwrap_or(text_width),
        height: known_dimensions.height.unwrap_or(text_height),
    }
}

struct TextRenderCtx<'a> {
    font: &'a mut yumeri_font::Font,
    glyph_cache: &'a mut yumeri_renderer::texture::glyph_cache::GlyphCache,
}

fn resolve_text_metrics(font_size: Option<f32>, line_height: Option<f32>) -> (f32, f32) {
    let fs = font_size.unwrap_or(DEFAULT_FONT_SIZE);
    (fs, line_height.unwrap_or(fs * DEFAULT_LINE_HEIGHT_FACTOR))
}

struct NodeVisualInfo {
    widget_type: WidgetType,
    visible: bool,
    background: Option<Color>,
    corner_radius: f32,
    opacity: f32,
    border_width: f32,
    texture_id: Option<yumeri_renderer::TextureId>,
    scroll_offset: Option<[f32; 2]>,
    scene_node: Option<yumeri_renderer::ui::NodeId>,
    children: Vec<UiNodeId>,
    // Layout (computed from taffy)
    abs_x: f32,
    abs_y: f32,
    w: f32,
    h: f32,
    z_index: i32,
    // Text-related (only meaningful for text-bearing widgets)
    text: Option<String>,
    text_color: Option<Color>,
    font_size: Option<f32>,
    line_height: Option<f32>,
}

fn sync_node_recursive(
    tree: &mut UiTree,
    scene: &mut Scene,
    text_ctx: &mut Option<TextRenderCtx>,
    node_id: UiNodeId,
    parent_x: f32,
    parent_y: f32,
    z_index: i32,
) {
    let mut info = {
        let node = match tree.nodes.get(node_id) {
            Some(n) => n,
            None => return,
        };
        let layout = tree.taffy.layout(node.taffy_node).expect("taffy layout");
        let is_text = node.widget_type.is_text_bearing();
        NodeVisualInfo {
            widget_type: node.widget_type,
            visible: node.style.visible,
            background: node.style.background,
            corner_radius: node.style.corner_radius,
            opacity: node.style.opacity,
            border_width: node.style.border_width,
            texture_id: node.props.texture_id,
            scroll_offset: node.props.scroll_offset,
            scene_node: node.scene_node,
            children: node.children.clone(),
            abs_x: parent_x + layout.location.x,
            abs_y: parent_y + layout.location.y,
            w: layout.size.width,
            h: layout.size.height,
            z_index,
            text: if is_text { node.props.text.clone() } else { None },
            text_color: if is_text { node.props.text_color } else { None },
            font_size: if is_text { node.props.font_size } else { None },
            line_height: if is_text { node.props.line_height } else { None },
        }
    };

    if !info.visible || info.w <= 0.0 || info.h <= 0.0 {
        if let Some(scene_id) = info.scene_node {
            scene.remove(scene_id);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = None;
            }
        }
        return;
    }

    let needs_scene_node = needs_visual(info.widget_type, info.background, info.border_width);

    if needs_scene_node {
        let shape_type = if info.widget_type == WidgetType::Image {
            ShapeType::Rect
        } else if info.corner_radius > 0.0 {
            ShapeType::RoundedRect
        } else {
            ShapeType::Rect
        };

        let scene_id = if let Some(id) = info.scene_node {
            id
        } else {
            let id = scene.add(shape_type);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = Some(id);
            }
            info.scene_node = Some(id);
            id
        };

        let cx = info.abs_x + info.w / 2.0;
        let cy = info.abs_y + info.h / 2.0;
        scene.set_position(scene_id, [cx, cy]);
        scene.set_size(scene_id, [info.w / 2.0, info.h / 2.0]);

        if let Some(bg) = info.background {
            scene.set_color(
                scene_id,
                Color::rgba(bg.r, bg.g, bg.b, bg.a * info.opacity),
            );
        } else if info.widget_type == WidgetType::Button {
            scene.set_color(
                scene_id,
                Color::rgba(DEFAULT_BUTTON_BG.r, DEFAULT_BUTTON_BG.g, DEFAULT_BUTTON_BG.b, info.opacity),
            );
        } else {
            scene.set_color(scene_id, Color::rgba(0.0, 0.0, 0.0, 0.0));
        }

        scene.set_corner_radius(scene_id, info.corner_radius);
        scene.set_z_index(scene_id, z_index);

        if let Some(tex_id) = info.texture_id {
            scene.set_texture(
                scene_id,
                Some(yumeri_renderer::Texture {
                    id: tex_id,
                    uv_rect: yumeri_renderer::UvRect::default(),
                }),
            );
        }
    } else if let Some(scene_id) = info.scene_node {
        // Don't remove Text scene nodes — they hold glyph children managed by render_text_if_needed
        if info.widget_type != WidgetType::Text {
            scene.remove(scene_id);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = None;
            }
            info.scene_node = None;
        }
    }

    // Render text inline (avoids separate full-tree pass)
    if let Some(text_ctx) = text_ctx.as_mut() {
        render_text_if_needed(tree, scene, text_ctx, node_id, &mut info);
    }

    // Recurse children
    let scroll_offset = info.scroll_offset.unwrap_or([0.0, 0.0]);
    let child_x = info.abs_x + scroll_offset[0];
    let child_y = info.abs_y + scroll_offset[1];

    for (i, child_id) in info.children.iter().enumerate() {
        sync_node_recursive(tree, scene, text_ctx, *child_id, child_x, child_y, z_index + 1 + i as i32);
    }
}

fn render_text_if_needed(
    tree: &mut UiTree,
    scene: &mut Scene,
    text_ctx: &mut TextRenderCtx,
    node_id: UiNodeId,
    info: &mut NodeVisualInfo,
) {
    let text = match &info.text {
        Some(t) if !t.is_empty() => t,
        _ => return,
    };

    let (font_size, line_height) = resolve_text_metrics(info.font_size, info.line_height);

    let text_style = yumeri_renderer::TextStyle {
        font_size,
        line_height,
        color: info.text_color.unwrap_or(Color::WHITE),
        max_width: Some(info.w),
        ..Default::default()
    };

    let parent_scene_node = if info.widget_type == WidgetType::Text {
        if let Some(id) = info.scene_node {
            id
        } else {
            let id = scene.add(ShapeType::None);
            if let Some(node) = tree.nodes.get_mut(node_id) {
                node.scene_node = Some(id);
            }
            info.scene_node = Some(id);
            id
        }
    } else {
        match info.scene_node {
            Some(id) => id,
            None => return,
        }
    };

    // Always update position, size, and z_index for Text (layout may have changed)
    if info.widget_type == WidgetType::Text {
        let cx = info.abs_x + info.w / 2.0;
        let cy = info.abs_y + info.h / 2.0;
        scene.set_position(parent_scene_node, [cx, cy]);
        scene.set_size(parent_scene_node, [info.w / 2.0, info.h / 2.0]);
        scene.set_z_index(parent_scene_node, info.z_index);
    }

    scene.set_text(parent_scene_node, text_ctx.font, text, &text_style, text_ctx.glyph_cache);
}

fn needs_visual(widget_type: WidgetType, background: Option<Color>, border_width: f32) -> bool {
    match widget_type {
        WidgetType::Container | WidgetType::Column | WidgetType::Row => {
            background.is_some() || border_width > 0.0
        }
        WidgetType::Stack => background.is_some(),
        WidgetType::Text => false,
        WidgetType::Button => true,
        WidgetType::Image => true,
        WidgetType::TextInput => true,
        WidgetType::Checkbox => true,
        WidgetType::ScrollView => false,
    }
}
