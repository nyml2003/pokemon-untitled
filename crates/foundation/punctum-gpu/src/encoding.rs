use crate::{InstanceData, PixelSize, RadarInstanceData, Viewport};

pub const UNIFORM_SIZE: u64 = 32;
pub const RADAR_INSTANCE_STRIDE: u64 = 128;

pub fn encode_instances(instances: &[InstanceData]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * crate::INSTANCE_STRIDE as usize);
    for instance in instances {
        for value in instance.grid_position {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in instance.grid_span {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in instance.pixel_offset {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in instance.atlas_rect {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&instance.tint);
        bytes.extend_from_slice(&instance.visible.to_le_bytes());
        for value in instance.corner_radii {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

pub fn encode_radar_instances(instances: &[RadarInstanceData]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(instances.len() * RADAR_INSTANCE_STRIDE as usize);
    for instance in instances {
        for value in instance.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(&instance.max.to_le_bytes());
        bytes.extend_from_slice(&instance.rings.to_le_bytes());
        for color in [
            instance.grid_color,
            instance.axis_color,
            instance.fill_color,
            instance.edge_color,
            instance.point_color,
        ] {
            for value in color {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        for value in instance.bounds {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

pub fn encode_uniform(viewport: Viewport, atlas_size: PixelSize) -> [u8; UNIFORM_SIZE as usize] {
    let mut bytes = [0; UNIFORM_SIZE as usize];
    let values = [
        viewport.target_size.width,
        viewport.target_size.height,
        viewport.origin.x as u32,
        viewport.origin.y as u32,
        viewport.cell_size.width,
        viewport.cell_size.height,
        atlas_size.width,
        atlas_size.height,
    ];
    for (chunk, value) in bytes.chunks_exact_mut(4).zip(values) {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}
