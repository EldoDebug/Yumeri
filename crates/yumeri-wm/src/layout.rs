use yumeri_types::ReservedRegions;

use crate::window::WindowId;

#[derive(Debug, Clone)]
pub struct LayoutConfig {
    pub output_size: (u32, u32),
    pub cascade_offset: (i32, i32),
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            output_size: (1280, 720),
            cascade_offset: (30, 30),
        }
    }
}

pub struct WindowLayout {
    pub position: (i32, i32),
    pub size: (u32, u32),
}

pub struct LayoutEngine {
    config: LayoutConfig,
    next_cascade_index: u32,
}

impl LayoutEngine {
    pub fn new(config: LayoutConfig) -> Self {
        Self {
            config,
            next_cascade_index: 0,
        }
    }

    pub fn config(&self) -> &LayoutConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut LayoutConfig {
        &mut self.config
    }

    pub fn set_output_size(&mut self, w: u32, h: u32) {
        self.config.output_size = (w, h);
    }

    pub fn allocate_initial(&mut self, size: (u32, u32), reserved: &ReservedRegions) -> WindowLayout {
        let layout = cascade_position(
            self.next_cascade_index,
            size,
            self.config.output_size,
            self.config.cascade_offset,
            reserved,
        );
        self.next_cascade_index += 1;
        layout
    }

    /// Calculate a default floating position without incrementing the cascade counter.
    pub fn default_position(&self, size: (u32, u32), reserved: &ReservedRegions) -> WindowLayout {
        cascade_position(
            0,
            size,
            self.config.output_size,
            self.config.cascade_offset,
            reserved,
        )
    }

    pub fn relayout(
        &self,
        windows: &[(WindowId, (u32, u32))],
        reserved: &ReservedRegions,
    ) -> Vec<(WindowId, WindowLayout)> {
        let (base_x, base_y, range_x, range_y) = cascade_area(self.config.output_size, reserved);
        let offset = self.config.cascade_offset;

        windows
            .iter()
            .enumerate()
            .map(|(i, &(wid, size))| {
                let dx = ((i as i32) * offset.0) % range_x;
                let dy = ((i as i32) * offset.1) % range_y;
                let layout = WindowLayout {
                    position: (base_x + dx, base_y + dy),
                    size,
                };
                (wid, layout)
            })
            .collect()
    }
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new(LayoutConfig::default())
    }
}

fn cascade_position(
    index: u32,
    size: (u32, u32),
    output_size: (u32, u32),
    offset: (i32, i32),
    reserved: &ReservedRegions,
) -> WindowLayout {
    let (base_x, base_y, range_x, range_y) = cascade_area(output_size, reserved);

    let dx = ((index as i32) * offset.0) % range_x;
    let dy = ((index as i32) * offset.1) % range_y;

    WindowLayout {
        position: (base_x + dx, base_y + dy),
        size,
    }
}

fn cascade_area(
    output_size: (u32, u32),
    reserved: &ReservedRegions,
) -> (i32, i32, i32, i32) {
    let area_x = reserved.left as i32;
    let area_y = reserved.top as i32;
    let area_w = output_size.0 as i32 - reserved.left as i32 - reserved.right as i32;
    let area_h = output_size.1 as i32 - reserved.top as i32 - reserved.bottom as i32;

    let base_x = area_x + 60;
    let base_y = area_y + 60;
    let max_x = (area_x + area_w - 200).max(base_x + 1);
    let max_y = (area_y + area_h - 200).max(base_y + 1);
    let range_x = (max_x - base_x).max(1);
    let range_y = (max_y - base_y).max(1);

    (base_x, base_y, range_x, range_y)
}
