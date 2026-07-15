use crate::{InstanceData, PixelSize, Viewport};

pub const UNIFORM_SIZE: u64 = 32;

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
