use slotmap::{new_key_type, SlotMap};
use taffy::TaffyTree;
use yumeri_animation::animator::Animator;
use yumeri_renderer::ui::NodeId as SceneNodeId;

use crate::callback::AnyCallback;
use crate::component::ComponentBox;
use crate::element::{ElementKey, WidgetProps, WidgetType};
use crate::event::{EventKind, EventPayload};
use crate::event::focus::FocusState;
use crate::style::Style;
use crate::template_provider::TemplateProvider;

new_key_type! { pub struct UiNodeId; }

pub(crate) struct UiNode {
    pub widget_type: WidgetType,
    pub style: Style,
    pub props: WidgetProps,
    pub parent: Option<UiNodeId>,
    pub children: Vec<UiNodeId>,
    pub taffy_node: taffy::NodeId,
    pub scene_node: Option<SceneNodeId>,
    pub event_handlers: Vec<(EventKind, AnyCallback)>,
    pub component: Option<ComponentBox>,
    pub focusable: bool,
    pub dirty: bool,
    pub key: Option<ElementKey>,
}

pub struct UiTree {
    pub(crate) nodes: SlotMap<UiNodeId, UiNode>,
    pub(crate) root: Option<UiNodeId>,
    pub(crate) taffy: TaffyTree<UiNodeId>,
    pub(crate) animator: Animator,
    pub(crate) needs_rebuild: bool,
    pub(crate) needs_layout: bool,
    pub(crate) focus: FocusState,
    pub(crate) viewport_size: (f32, f32),
    pub(crate) cursor_pos: (f32, f32),
    pub(crate) hovered_node: Option<UiNodeId>,
    pub(crate) pending_scene_removals: Vec<SceneNodeId>,
    pub(crate) template_provider: Option<Box<dyn TemplateProvider>>,
}

impl UiTree {
    pub fn new() -> Self {
        Self {
            nodes: SlotMap::with_key(),
            root: None,
            taffy: TaffyTree::new(),
            animator: Animator::new(),
            needs_rebuild: true,
            needs_layout: true,
            focus: FocusState::new(),
            viewport_size: (800.0, 600.0),
            cursor_pos: (0.0, 0.0),
            hovered_node: None,
            pending_scene_removals: Vec::new(),
            template_provider: None,
        }
    }

    pub fn set_template_provider(&mut self, provider: impl TemplateProvider + 'static) {
        self.template_provider = Some(Box::new(provider));
    }

    pub fn template_provider(&self) -> Option<&dyn TemplateProvider> {
        self.template_provider.as_deref()
    }

    pub(crate) fn template_provider_ptr(&self) -> Option<*const dyn TemplateProvider> {
        self.template_provider.as_deref()
            .map(|p| p as *const dyn TemplateProvider)
    }

    pub fn set_viewport_size(&mut self, width: f32, height: f32) {
        self.viewport_size = (width, height);
        self.needs_layout = true;
    }

    pub fn request_rebuild(&mut self) {
        self.needs_rebuild = true;
    }

    pub fn animator(&mut self) -> &mut Animator {
        &mut self.animator
    }

    pub fn cursor_pos(&self) -> (f32, f32) {
        self.cursor_pos
    }

    pub fn root(&self) -> Option<UiNodeId> {
        self.root
    }

    pub(crate) fn insert_node(
        &mut self,
        widget_type: WidgetType,
        style: Style,
        props: WidgetProps,
        parent: Option<UiNodeId>,
        focusable: bool,
        key: Option<ElementKey>,
    ) -> UiNodeId {
        let taffy_style = crate::layout::to_taffy_style(&style);
        let taffy_node = self.taffy.new_leaf(taffy_style).expect("taffy new_leaf");

        let id = self.nodes.insert(UiNode {
            widget_type,
            style,
            props,
            parent,
            children: Vec::new(),
            taffy_node,
            scene_node: None,
            event_handlers: Vec::new(),
            component: None,
            focusable,
            dirty: true,
            key,
        });

        self.taffy
            .set_node_context(taffy_node, Some(id))
            .expect("taffy set_node_context");

        if let Some(parent_id) = parent {
            if let Some(parent_node) = self.nodes.get_mut(parent_id) {
                parent_node.children.push(id);
            }
            if let Some(parent_taffy) = self.nodes.get(parent_id).map(|n| n.taffy_node) {
                let children: Vec<_> = self.nodes[parent_id]
                    .children
                    .iter()
                    .map(|&c| self.nodes[c].taffy_node)
                    .collect();
                self.taffy
                    .set_children(parent_taffy, &children)
                    .expect("taffy set_children");
            }
        }

        id
    }

    pub(crate) fn remove_node(&mut self, id: UiNodeId) {
        let children: Vec<UiNodeId> = self
            .nodes
            .get(id)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        for child_id in children {
            self.remove_node(child_id);
        }

        if let Some(node) = self.nodes.remove(id) {
            let _ = self.taffy.remove(node.taffy_node);

            // Queue scene node for removal so the renderer cleans it up
            if let Some(scene_id) = node.scene_node {
                self.pending_scene_removals.push(scene_id);
            }

            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(parent_id) {
                    parent.children.retain(|&c| c != id);
                }
            }
        }
    }

    pub(crate) fn invoke_callback(
        &mut self,
        node_id: UiNodeId,
        event_kind: EventKind,
        payload: &EventPayload,
    ) -> bool {
        // Check if this specific node has a handler for the event kind
        let handler_idx = match self.nodes.get(node_id) {
            Some(node) => {
                match node
                    .event_handlers
                    .iter()
                    .position(|(kind, _)| *kind == event_kind)
                {
                    Some(idx) => idx,
                    None => return false,
                }
            }
            None => return false,
        };

        // Find the component that owns this callback
        let owner_node_id = self.find_owner_component(node_id);

        if let Some(owner_id) = owner_node_id {
            // Take the component out to avoid borrow conflicts
            let component = self
                .nodes
                .get_mut(owner_id)
                .and_then(|n| n.component.as_mut())
                .and_then(|c| c.take());

            if let Some(mut inner) = component {
                // Take the handler out, invoke, put it back
                let mut handler = self.nodes[node_id].event_handlers.remove(handler_idx);
                handler.1.invoke(inner.as_mut(), payload);
                self.nodes[node_id]
                    .event_handlers
                    .insert(handler_idx, handler);

                // Put component back
                if let Some(node) = self.nodes.get_mut(owner_id) {
                    if let Some(comp) = &mut node.component {
                        comp.put_back(inner);
                    }
                }
                self.needs_rebuild = true;
                return true;
            }
        }

        false
    }

    pub(crate) fn compute_absolute_position(&self, node_id: UiNodeId) -> Option<(f32, f32)> {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut current = Some(node_id);
        while let Some(id) = current {
            let node = self.nodes.get(id)?;
            let layout = self.taffy.layout(node.taffy_node).ok()?;
            x += layout.location.x;
            y += layout.location.y;
            current = node.parent;
        }
        Some((x, y))
    }

    fn find_owner_component(&self, start: UiNodeId) -> Option<UiNodeId> {
        let mut current = Some(start);
        while let Some(id) = current {
            if let Some(node) = self.nodes.get(id) {
                if node.component.is_some() {
                    return Some(id);
                }
                current = node.parent;
            } else {
                break;
            }
        }
        None
    }
}

impl Default for UiTree {
    fn default() -> Self {
        Self::new()
    }
}
