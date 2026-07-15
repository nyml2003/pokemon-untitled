use punctum_gpu::{
    INSTANCE_STRIDE, InstanceData, PixelOffset, PixelSize, UNIFORM_SIZE, Viewport,
    encode_instances, encode_uniform,
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
