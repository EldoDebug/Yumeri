use super::scene::Scene;

pub struct UiContext<'a> {
    scene: &'a mut Scene,
    surface_size: (u32, u32),
}

impl<'a> UiContext<'a> {
    pub fn new(scene: &'a mut Scene, surface_size: (u32, u32)) -> Self {
        Self {
            scene,
            surface_size,
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }
}
