use punctum_gpu::{
    INSTANCE_STRIDE, InstanceData, PixelOffset, PixelSize, RADAR_INSTANCE_STRIDE,
    RadarInstanceData, Rgba8, UNIFORM_SIZE, Viewport, encode_instances, encode_radar_instances,
    encode_uniform,
};

#[test]
fn instance_encoding_matches_the_declared_vertex_stride() {
    let instance = InstanceData {
        grid_position: [1, 2],
        grid_span: [12, 13],
        pixel_offset: [-14, 15],
        atlas_rect: [3, 4, 5, 6],
        tint: [7, 8, 9, 10],
        visible: 11,
        corner_radii: [12, 13, 14, 15],
    };
    let bytes = encode_instances(&[instance]);

    assert_eq!(bytes.len(), INSTANCE_STRIDE as usize);
    assert_eq!(&bytes[0..8], &[1, 0, 0, 0, 2, 0, 0, 0]);
    assert_eq!(&bytes[8..16], &[12, 0, 0, 0, 13, 0, 0, 0]);
    assert_eq!(&bytes[16..20], &(-14_i32).to_le_bytes());
    assert_eq!(&bytes[20..24], &15_i32.to_le_bytes());
    assert_eq!(&bytes[40..44], &[7, 8, 9, 10]);
    assert_eq!(&bytes[44..48], &[11, 0, 0, 0]);
    assert_eq!(&bytes[48..52], &[12, 0, 0, 0]);
    assert_eq!(&bytes[60..64], &[15, 0, 0, 0]);
}

#[test]
fn uniform_encoding_preserves_signed_origin_bits() {
    let viewport = Viewport::new(
        PixelSize::new(100, 80),
        PixelOffset::new(-2, 3),
        PixelSize::new(8, 9),
    )
    .unwrap();
    let bytes = encode_uniform(viewport, PixelSize::new(64, 32));

    assert_eq!(bytes.len(), UNIFORM_SIZE as usize);
    assert_eq!(&bytes[8..12], &(-2_i32).to_le_bytes());
    assert_eq!(&bytes[28..32], &32_u32.to_le_bytes());
}

#[test]
fn radar_encoding_matches_the_declared_storage_stride() {
    let radar = RadarInstanceData::new(
        [1, 2, 3, 4, 5, 6],
        256,
        4,
        [
            Rgba8::new(1, 2, 3, 4),
            Rgba8::new(5, 6, 7, 8),
            Rgba8::new(9, 10, 11, 12),
            Rgba8::new(13, 14, 15, 16),
            Rgba8::new(17, 18, 19, 20),
        ],
    );
    let bytes = encode_radar_instances(&[radar]);

    assert_eq!(bytes.len(), RADAR_INSTANCE_STRIDE as usize);
    assert_eq!(&bytes[0..4], &1_u32.to_le_bytes());
    assert_eq!(&bytes[20..24], &6_u32.to_le_bytes());
    assert_eq!(&bytes[24..28], &256_u32.to_le_bytes());
    assert_eq!(&bytes[32..36], &1_u32.to_le_bytes());
    assert_eq!(&bytes[96..100], &17_u32.to_le_bytes());
    assert_eq!(&bytes[108..112], &20_u32.to_le_bytes());
    assert_eq!(&bytes[112..116], &0_u32.to_le_bytes());
    assert_eq!(&bytes[124..128], &0_u32.to_le_bytes());
}
