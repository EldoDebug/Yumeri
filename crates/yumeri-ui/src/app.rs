use std::time::Duration;

use yumeri_input::{
    ButtonState, InputEvent, Key, KeyboardEvent, MouseButton, NamedKey, PointerEvent,
    PointerEventKind,
};
use yumeri_renderer::texture::glyph_cache::GlyphCache;
use yumeri_renderer::ui::Scene;

use crate::component::Component;
use crate::event::EventPayload;
use crate::event::focus::{focus_next, focus_prev};
use crate::event::hit_test::hit_test;
use crate::event::propagation::dispatch_event;
use crate::reconciler::{mount_root_component, rebuild_component};
use crate::renderer_bridge::sync_to_scene;
use crate::template_provider::TemplateProvider;
use crate::tree::UiTree;

pub struct UiApp<C: Component> {
    create: Option<Box<dyn FnOnce() -> C>>,
    tree: UiTree,
    font: Option<yumeri_font::Font>,
    last_frame: Option<std::time::Instant>,
}

impl<C: Component> UiApp<C> {
    pub fn new(create: impl FnOnce() -> C + 'static) -> Self {
        Self {
            create: Some(Box::new(create)),
            tree: UiTree::new(),
            font: None,
            last_frame: None,
        }
    }
}

impl<C: Component> UiApp<C> {
    pub fn setup(
        &mut self,
        scene: &mut Scene,
        surface_size: (u32, u32),
        glyph_cache: Option<&mut GlyphCache>,
    ) {
        self.tree
            .set_viewport_size(surface_size.0 as f32, surface_size.1 as f32);

        self.font = Some(yumeri_font::Font::new());

        if let Some(create) = self.create.take() {
            let component = create();
            mount_root_component(&mut self.tree, component);
            sync_to_scene(&mut self.tree, scene, self.font.as_mut(), glyph_cache);
        }
    }

    pub fn tick(&mut self, scene: &mut Scene, glyph_cache: Option<&mut GlyphCache>) {
        let now = std::time::Instant::now();
        let dt = self
            .last_frame
            .map(|last| now.duration_since(last))
            .unwrap_or(Duration::ZERO);
        self.last_frame = Some(now);

        // Update animations
        self.tree.animator.update(dt);
        self.tree.animator.gc();

        // Only rebuild when animation values actually changed
        if self.tree.animator.had_value_change() {
            self.tree.needs_rebuild = true;
        }

        // Rebuild dirty components
        if self.tree.needs_rebuild {
            if let Some(root) = self.tree.root {
                rebuild_component(&mut self.tree, root);
            }
            self.tree.needs_rebuild = false;
            self.tree.needs_layout = true;
        }

        // Sync to scene (includes text rendering)
        if self.tree.needs_layout {
            sync_to_scene(&mut self.tree, scene, self.font.as_mut(), glyph_cache);
            self.tree.needs_layout = false;
        }
    }

    pub fn on_resize(&mut self, width: u32, height: u32) {
        self.tree.set_viewport_size(width as f32, height as f32);
    }

    pub fn on_input(&mut self, event: &InputEvent) {
        match event {
            InputEvent::Keyboard(kb) => {
                if kb.state == ButtonState::Pressed {
                    self.handle_key_down(kb);
                }
            }
            InputEvent::Pointer(ptr) => self.handle_pointer(ptr),
            InputEvent::FocusChanged(_) => {}
        }
    }

    fn handle_key_down(&mut self, kb: &KeyboardEvent) {
        if kb.key == Key::Named(NamedKey::Tab) {
            if kb.modifiers.shift {
                focus_prev(&mut self.tree);
            } else {
                focus_next(&mut self.tree);
            }
            return;
        }

        // Dispatch text input for character keys
        if let Some(ref text) = kb.text {
            if !text.is_empty() && !kb.modifiers.ctrl && !kb.modifiers.alt && !kb.modifiers.meta {
                if let Some(focused) = self.tree.focus.focused() {
                    dispatch_event(
                        &mut self.tree,
                        focused,
                        &EventPayload::TextInput {
                            text: text.clone(),
                        },
                    );
                }
            }
        }

        if let Some(focused) = self.tree.focus.focused() {
            dispatch_event(
                &mut self.tree,
                focused,
                &EventPayload::KeyDown {
                    event: kb.clone(),
                },
            );
        }
    }

    fn handle_pointer(&mut self, ptr: &PointerEvent) {
        match &ptr.kind {
            PointerEventKind::Moved => {
                let x = ptr.position.0 as f32;
                let y = ptr.position.1 as f32;
                self.tree.cursor_pos = (x, y);

                let new_hovered = hit_test(&self.tree, x, y);

                if new_hovered != self.tree.hovered_node {
                    if let Some(old) = self.tree.hovered_node {
                        dispatch_event(&mut self.tree, old, &EventPayload::MouseLeave);
                    }
                    if let Some(new) = new_hovered {
                        dispatch_event(&mut self.tree, new, &EventPayload::MouseEnter);
                    }
                    self.tree.hovered_node = new_hovered;
                }
            }
            PointerEventKind::ButtonPressed(button) => {
                let (x, y) = self.tree.cursor_pos;
                let target = hit_test(&self.tree, x, y);

                if *button == MouseButton::Left {
                    if let Some(t) = target {
                        let focusable = self
                            .tree
                            .nodes
                            .get(t)
                            .map(|n| n.focusable)
                            .unwrap_or(false);
                        if focusable {
                            self.tree.focus.set_focus(Some(t));
                        }

                        dispatch_event(&mut self.tree, t, &EventPayload::Click);
                    }
                }

                if let Some(t) = target {
                    dispatch_event(
                        &mut self.tree,
                        t,
                        &EventPayload::MouseDown {
                            x,
                            y,
                            button: *button,
                        },
                    );
                }
            }
            PointerEventKind::ButtonReleased(button) => {
                let (x, y) = self.tree.cursor_pos;
                if let Some(target) = hit_test(&self.tree, x, y) {
                    dispatch_event(
                        &mut self.tree,
                        target,
                        &EventPayload::MouseUp {
                            x,
                            y,
                            button: *button,
                        },
                    );
                }
            }
            PointerEventKind::Scroll { delta_x, delta_y } => {
                let (x, y) = self.tree.cursor_pos;
                if let Some(target) = hit_test(&self.tree, x, y) {
                    dispatch_event(
                        &mut self.tree,
                        target,
                        &EventPayload::Scroll {
                            delta_x: *delta_x as f32,
                            delta_y: *delta_y as f32,
                        },
                    );
                }
            }
            PointerEventKind::Entered | PointerEventKind::Left => {}
        }
    }

    pub fn tree(&self) -> &UiTree {
        &self.tree
    }

    pub fn tree_mut(&mut self) -> &mut UiTree {
        &mut self.tree
    }

    pub fn set_template_provider(&mut self, provider: impl TemplateProvider + 'static) {
        self.tree.set_template_provider(provider);
    }
}
