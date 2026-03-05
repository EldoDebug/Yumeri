use super::pass::PassNode;
use super::resource::{ResourceId, VirtualResource};

#[allow(dead_code)]
pub(crate) struct CompiledGraph {
    pub passes: Vec<PassNode>,
    pub backbuffer: Option<ResourceId>,
}

impl CompiledGraph {
    pub fn compile(
        passes: Vec<PassNode>,
        _resources: Vec<VirtualResource>,
        backbuffer: Option<ResourceId>,
    ) -> Self {
        // Passes execute in submission order; barriers are handled by the executor.
        // Future: topological sort, pass culling, resource aliasing.
        Self { passes, backbuffer }
    }
}
