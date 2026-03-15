use std::collections::HashMap;

use crate::component::ComponentBox;
use crate::element::{Element, ElementKey, ElementKind, WidgetType};
use crate::event_ctx::EventCtx;
use crate::style;
use crate::tree::{UiNodeId, UiTree};
use crate::view_ctx::ViewCtx;

pub(crate) fn reconcile(tree: &mut UiTree, parent: Option<UiNodeId>, new_elements: Vec<Element>) {
    let old_children: Vec<UiNodeId> = match parent {
        Some(pid) => tree
            .nodes
            .get(pid)
            .map(|n| n.children.clone())
            .unwrap_or_default(),
        None => tree.root.into_iter().collect(),
    };

    // Build O(1) lookup from key → node id using stored keys
    let mut old_by_key: HashMap<ElementKey, UiNodeId> = HashMap::with_capacity(old_children.len());
    for (i, &id) in old_children.iter().enumerate() {
        let key = tree
            .nodes
            .get(id)
            .and_then(|n| n.key.clone())
            .unwrap_or(ElementKey::Index(i));
        old_by_key.insert(key, id);
    }

    let mut new_child_ids = Vec::with_capacity(new_elements.len());

    for (new_idx, element) in new_elements.into_iter().enumerate() {
        let new_key = element
            .key
            .clone()
            .unwrap_or(ElementKey::Index(new_idx));

        let matched = old_by_key.remove(&new_key);

        match element.kind {
            ElementKind::Widget(widget_elem) => {
                if let Some(old_id) = matched {
                    let type_matches = tree
                        .nodes
                        .get(old_id)
                        .map(|n| n.widget_type == widget_elem.widget_type)
                        .unwrap_or(false);

                    if type_matches {
                        // Update existing node
                        if let Some(node) = tree.nodes.get_mut(old_id) {
                            node.style = widget_elem.style;
                            node.props = widget_elem.props;
                            node.event_handlers = widget_elem.event_handlers;
                            node.focusable = widget_elem.focusable;

                            let taffy_style = crate::layout::to_taffy_style(&node.style);
                            let taffy_node = node.taffy_node;
                            tree.taffy
                                .set_style(taffy_node, taffy_style)
                                .expect("taffy set_style");
                        }

                        reconcile(tree, Some(old_id), widget_elem.children);
                        new_child_ids.push(old_id);
                    } else {
                        // Type mismatch: remove old, mount new
                        tree.remove_node(old_id);
                        let wt = widget_elem.widget_type;
                        let new_id = mount_widget(tree, parent, wt, widget_elem, Some(new_key));
                        new_child_ids.push(new_id);
                    }
                } else {
                    // No match: mount new
                    let wt = widget_elem.widget_type;
                    let new_id = mount_widget(tree, parent, wt, widget_elem, Some(new_key));
                    new_child_ids.push(new_id);
                }
            }
            ElementKind::Component(comp_elem) => {
                if let Some(old_id) = matched {
                    let type_matches = tree
                        .nodes
                        .get(old_id)
                        .and_then(|n| n.component.as_ref())
                        .map(|c| c.type_id() == comp_elem.type_id)
                        .unwrap_or(false);

                    if type_matches {
                        rebuild_component(tree, old_id);
                        new_child_ids.push(old_id);
                    } else {
                        unmount_component(tree, old_id);
                        tree.remove_node(old_id);
                        let new_id = mount_component(tree, parent, comp_elem.create, Some(new_key));
                        new_child_ids.push(new_id);
                    }
                } else {
                    let new_id = mount_component(tree, parent, comp_elem.create, Some(new_key));
                    new_child_ids.push(new_id);
                }
            }
        }
    }

    // Unmount remaining old nodes
    for (_, old_id) in old_by_key {
        unmount_component(tree, old_id);
        tree.remove_node(old_id);
    }

    // Update parent's children list and sync taffy
    match parent {
        Some(pid) => {
            // Build taffy children from new_child_ids before moving it
            let taffy_children: Vec<_> = new_child_ids
                .iter()
                .filter_map(|&id| tree.nodes.get(id).map(|n| n.taffy_node))
                .collect();

            if let Some(parent_node) = tree.nodes.get_mut(pid) {
                parent_node.children = new_child_ids;
            }
            if let Some(parent_taffy) = tree.nodes.get(pid).map(|n| n.taffy_node) {
                tree.taffy
                    .set_children(parent_taffy, &taffy_children)
                    .expect("taffy set_children");
            }
        }
        None => {
            tree.root = new_child_ids.into_iter().next();
        }
    }

    tree.needs_layout = true;
}

fn mount_widget(
    tree: &mut UiTree,
    parent: Option<UiNodeId>,
    widget_type: WidgetType,
    widget_elem: crate::element::WidgetElement,
    key: Option<ElementKey>,
) -> UiNodeId {
    let id = tree.insert_node(
        widget_type,
        widget_elem.style,
        widget_elem.props,
        parent,
        widget_elem.focusable,
        key,
    );
    if let Some(node) = tree.nodes.get_mut(id) {
        node.event_handlers = widget_elem.event_handlers;
    }

    let children = widget_elem.children;
    if !children.is_empty() {
        reconcile(tree, Some(id), children);
    }
    id
}

/// Shared logic: insert a container node, store the component, call on_mount, run view, reconcile.
fn init_component_node(
    tree: &mut UiTree,
    comp_box: ComponentBox,
    parent: Option<UiNodeId>,
    container_style: style::Style,
    key: Option<ElementKey>,
) -> UiNodeId {
    let comp_type_id = comp_box.type_id();

    let id = tree.insert_node(
        WidgetType::Container,
        container_style,
        Default::default(),
        parent,
        false,
        key,
    );

    if let Some(node) = tree.nodes.get_mut(id) {
        node.component = Some(comp_box);
    }

    // Call on_mount
    {
        let mut ctx = EventCtx {
            animator: &mut tree.animator,
        };
        if let Some(node) = tree.nodes.get_mut(id) {
            if let Some(comp) = &mut node.component {
                comp.on_mount(&mut ctx);
            }
        }
    }

    // Run initial view
    let element = {
        let node = match tree.nodes.get(id) {
            Some(n) => n,
            None => return id,
        };
        let comp = match &node.component {
            Some(c) => c,
            None => return id,
        };
        let mut ctx = ViewCtx::new(comp_type_id, &mut tree.animator)
            .with_template_provider_ptr(tree.template_provider_ptr());
        comp.view(&mut ctx)
    };

    reconcile(tree, Some(id), vec![element]);
    id
}

fn mount_component(
    tree: &mut UiTree,
    parent: Option<UiNodeId>,
    create: Box<dyn FnOnce() -> ComponentBox>,
    key: Option<ElementKey>,
) -> UiNodeId {
    init_component_node(
        tree,
        create(),
        parent,
        style::Style {
            width: style::Dimension::Percent(1.0),
            height: style::Dimension::Auto,
            ..Default::default()
        },
        key,
    )
}

fn unmount_component(tree: &mut UiTree, id: UiNodeId) {
    if let Some(node) = tree.nodes.get_mut(id) {
        if let Some(mut comp) = node.component.take() {
            let mut ctx = EventCtx {
                animator: &mut tree.animator,
            };
            comp.on_unmount(&mut ctx);
        }
    }
}

pub(crate) fn rebuild_component(tree: &mut UiTree, id: UiNodeId) {
    let element = {
        let node = match tree.nodes.get(id) {
            Some(n) => n,
            None => return,
        };
        let comp = match &node.component {
            Some(c) => c,
            None => return,
        };

        let type_id = comp.type_id();
        let mut ctx = ViewCtx::new(type_id, &mut tree.animator)
            .with_template_provider_ptr(tree.template_provider_ptr());
        comp.view(&mut ctx)
    };

    reconcile(tree, Some(id), vec![element]);
}

pub(crate) fn mount_root_component<C: crate::component::Component>(
    tree: &mut UiTree,
    component: C,
) {
    let comp_box = ComponentBox::new(component);

    let id = init_component_node(
        tree,
        comp_box,
        None,
        style::Style {
            width: style::Dimension::Percent(1.0),
            height: style::Dimension::Percent(1.0),
            ..Default::default()
        },
        None,
    );

    tree.root = Some(id);
    tree.needs_rebuild = false;
    tree.needs_layout = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::element::*;
    use crate::style::Style;
    use crate::view_ctx::ViewCtx;
    use crate::widget::{Column, Text, Container};

    struct TestComponent {
        count: i32,
    }

    impl Component for TestComponent {
        fn view(&self, ctx: &mut ViewCtx) -> Element {
            Column::new()
                .child(Text::new(format!("Count: {}", self.count)))
                .child(
                    Container::new()
                        .on_click(ctx.callback(|this: &mut Self, _| {
                            this.count += 1;
                        }))
                        .child(Text::new("Click")),
                )
                .into()
        }
    }

    #[test]
    fn mount_root_creates_tree() {
        let mut tree = UiTree::new();
        mount_root_component(&mut tree, TestComponent { count: 0 });

        assert!(tree.root.is_some());
        // Root (component container) + Column + Text + Container + Text = 5 nodes
        assert!(tree.nodes.len() >= 5);
    }

    #[test]
    fn rebuild_preserves_component_state() {
        let mut tree = UiTree::new();
        mount_root_component(&mut tree, TestComponent { count: 42 });

        let root = tree.root.unwrap();

        // Rebuild should preserve the component
        rebuild_component(&mut tree, root);

        let node = tree.nodes.get(root).unwrap();
        assert!(node.component.is_some());
    }

    #[test]
    fn reconcile_adds_new_children() {
        let mut tree = UiTree::new();
        let parent = tree.insert_node(
            WidgetType::Container,
            Style::default(),
            WidgetProps::default(),
            None,
            false,
            None,
        );
        tree.root = Some(parent);

        let elements = vec![
            Text::new("Hello").into(),
            Text::new("World").into(),
        ];
        reconcile(&mut tree, Some(parent), elements);

        let parent_node = tree.nodes.get(parent).unwrap();
        assert_eq!(parent_node.children.len(), 2);
    }

    #[test]
    fn reconcile_removes_old_children() {
        let mut tree = UiTree::new();
        let parent = tree.insert_node(
            WidgetType::Container,
            Style::default(),
            WidgetProps::default(),
            None,
            false,
            None,
        );
        tree.root = Some(parent);

        // Add 3 children
        let elements = vec![
            Text::new("A").into(),
            Text::new("B").into(),
            Text::new("C").into(),
        ];
        reconcile(&mut tree, Some(parent), elements);
        assert_eq!(tree.nodes.get(parent).unwrap().children.len(), 3);

        // Reconcile with only 1 child - should remove 2
        let elements = vec![Text::new("A").into()];
        reconcile(&mut tree, Some(parent), elements);
        assert_eq!(tree.nodes.get(parent).unwrap().children.len(), 1);
    }

    #[test]
    fn reconcile_updates_existing_same_type() {
        let mut tree = UiTree::new();
        let parent = tree.insert_node(
            WidgetType::Container,
            Style::default(),
            WidgetProps::default(),
            None,
            false,
            None,
        );
        tree.root = Some(parent);

        let elements = vec![Text::new("Old").into()];
        reconcile(&mut tree, Some(parent), elements);

        let child_id = tree.nodes.get(parent).unwrap().children[0];

        // Reconcile with same type but different content
        let elements = vec![Text::new("New").into()];
        reconcile(&mut tree, Some(parent), elements);

        // Same node should be reused
        let new_child_id = tree.nodes.get(parent).unwrap().children[0];
        assert_eq!(child_id, new_child_id);

        // Props should be updated
        let node = tree.nodes.get(child_id).unwrap();
        assert_eq!(node.props.text.as_deref(), Some("New"));
    }

    #[test]
    fn reconcile_named_key_reorders_without_remount() {
        let mut tree = UiTree::new();
        let parent = tree.insert_node(
            WidgetType::Container,
            Style::default(),
            WidgetProps::default(),
            None,
            false,
            None,
        );
        tree.root = Some(parent);

        // Mount A, B with named keys
        let elements = vec![
            Element {
                key: Some(ElementKey::Named("a".into())),
                kind: ElementKind::Widget(WidgetElement {
                    widget_type: WidgetType::Text,
                    style: Style::default(),
                    props: WidgetProps { text: Some("A".into()), ..Default::default() },
                    children: Vec::new(),
                    event_handlers: Vec::new(),
                    focusable: false,
                }),
            },
            Element {
                key: Some(ElementKey::Named("b".into())),
                kind: ElementKind::Widget(WidgetElement {
                    widget_type: WidgetType::Text,
                    style: Style::default(),
                    props: WidgetProps { text: Some("B".into()), ..Default::default() },
                    children: Vec::new(),
                    event_handlers: Vec::new(),
                    focusable: false,
                }),
            },
        ];
        reconcile(&mut tree, Some(parent), elements);

        let children = tree.nodes.get(parent).unwrap().children.clone();
        assert_eq!(children.len(), 2);
        let id_a = children[0];
        let id_b = children[1];

        // Reorder: B, A
        let elements = vec![
            Element {
                key: Some(ElementKey::Named("b".into())),
                kind: ElementKind::Widget(WidgetElement {
                    widget_type: WidgetType::Text,
                    style: Style::default(),
                    props: WidgetProps { text: Some("B2".into()), ..Default::default() },
                    children: Vec::new(),
                    event_handlers: Vec::new(),
                    focusable: false,
                }),
            },
            Element {
                key: Some(ElementKey::Named("a".into())),
                kind: ElementKind::Widget(WidgetElement {
                    widget_type: WidgetType::Text,
                    style: Style::default(),
                    props: WidgetProps { text: Some("A2".into()), ..Default::default() },
                    children: Vec::new(),
                    event_handlers: Vec::new(),
                    focusable: false,
                }),
            },
        ];
        reconcile(&mut tree, Some(parent), elements);

        let children_after = tree.nodes.get(parent).unwrap().children.clone();
        assert_eq!(children_after.len(), 2);
        // Node IDs should be preserved (same nodes, just reordered)
        assert_eq!(children_after[0], id_b);
        assert_eq!(children_after[1], id_a);
        // Props should be updated
        assert_eq!(tree.nodes.get(id_b).unwrap().props.text.as_deref(), Some("B2"));
        assert_eq!(tree.nodes.get(id_a).unwrap().props.text.as_deref(), Some("A2"));
    }
}
