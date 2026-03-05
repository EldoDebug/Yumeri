use super::pass::{PassId, PassNode, RenderPassContext};
use super::resource::{ImageDesc, ResourceDesc, ResourceId, VirtualResource};

pub struct RenderGraphBuilder {
    resources: Vec<VirtualResource>,
    passes: Vec<PassNode>,
    next_resource_id: u32,
    next_pass_id: u32,
    backbuffer: Option<ResourceId>,
}

impl RenderGraphBuilder {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            passes: Vec::new(),
            next_resource_id: 0,
            next_pass_id: 0,
            backbuffer: None,
        }
    }

    pub fn import_backbuffer(&mut self) -> ResourceId {
        let id = ResourceId(self.next_resource_id);
        self.next_resource_id += 1;
        self.resources.push(VirtualResource {
            id,
            desc: ResourceDesc::Swapchain,
            written_by: Vec::new(),
            read_by: Vec::new(),
        });
        self.backbuffer = Some(id);
        id
    }

    #[allow(dead_code)]
    pub fn create_image(&mut self, desc: ImageDesc) -> ResourceId {
        let id = ResourceId(self.next_resource_id);
        self.next_resource_id += 1;
        self.resources.push(VirtualResource {
            id,
            desc: ResourceDesc::Image(desc),
            written_by: Vec::new(),
            read_by: Vec::new(),
        });
        id
    }

    pub fn add_pass<F, E>(&mut self, name: &str, setup: F) -> PassId
    where
        F: FnOnce(&mut PassBuilder) -> E,
        E: FnOnce(&mut RenderPassContext) + 'static,
    {
        let id = PassId(self.next_pass_id);
        self.next_pass_id += 1;

        let mut pass_builder = PassBuilder {
            reads: Vec::new(),
            writes: Vec::new(),
        };
        let execute_fn = setup(&mut pass_builder);

        for &res_id in &pass_builder.reads {
            if let Some(res) = self.resources.get_mut(res_id.0 as usize) {
                res.read_by.push(id);
            }
        }
        for &res_id in &pass_builder.writes {
            if let Some(res) = self.resources.get_mut(res_id.0 as usize) {
                res.written_by.push(id);
            }
        }

        self.passes.push(PassNode {
            id,
            name: name.to_string(),
            reads: pass_builder.reads,
            writes: pass_builder.writes,
            execute_fn: Some(Box::new(execute_fn)),
        });

        id
    }

    pub(crate) fn build(self) -> (Vec<PassNode>, Vec<VirtualResource>, Option<ResourceId>) {
        (self.passes, self.resources, self.backbuffer)
    }
}

pub struct PassBuilder {
    pub(crate) reads: Vec<ResourceId>,
    pub(crate) writes: Vec<ResourceId>,
}

impl PassBuilder {
    #[allow(dead_code)]
    pub fn read(&mut self, resource: ResourceId) {
        self.reads.push(resource);
    }

    pub fn write(&mut self, resource: ResourceId) {
        self.writes.push(resource);
    }
}
