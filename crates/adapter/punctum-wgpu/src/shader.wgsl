struct ViewportUniform {
    target_size: vec2<u32>,
    origin: vec2<i32>,
    cell_size: vec2<u32>,
    atlas_size: vec2<u32>,
}

@group(0) @binding(0)
var<uniform> viewport: ViewportUniform;

@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

struct VertexInput {
    @location(0) grid_position: vec2<u32>,
    @location(1) grid_span: vec2<u32>,
    @location(2) pixel_offset: vec2<i32>,
    @location(3) atlas_rect: vec4<u32>,
    @location(4) tint: vec4<f32>,
    @location(5) visible: u32,
    @location(6) corner_radii: vec4<u32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) local_pixel: vec2<f32>,
    @location(3) pixel_size: vec2<f32>,
    @location(4) corner_radii: vec4<f32>,
}

const QUAD: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(0.0, 1.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
);

@vertex
fn vs_main(input: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corner = QUAD[vertex_index];
    let pixel_origin = vec2<f32>(viewport.origin)
        + vec2<f32>(input.grid_position * viewport.cell_size)
        + vec2<f32>(input.pixel_offset);
    let pixel = pixel_origin + corner * vec2<f32>(input.grid_span * viewport.cell_size);
    let target_size = vec2<f32>(viewport.target_size);
    let ndc = vec2<f32>(
        pixel.x / target_size.x * 2.0 - 1.0,
        1.0 - pixel.y / target_size.y * 2.0,
    );
    let atlas_pixel = vec2<f32>(input.atlas_rect.xy)
        + corner * vec2<f32>(input.atlas_rect.zw);

    var output: VertexOutput;
    output.position = vec4<f32>(ndc, 0.0, 1.0);
    output.uv = atlas_pixel / vec2<f32>(viewport.atlas_size);
    output.tint = input.tint * f32(input.visible);
    output.local_pixel = corner * vec2<f32>(input.grid_span * viewport.cell_size);
    output.pixel_size = vec2<f32>(input.grid_span * viewport.cell_size);
    output.corner_radii = vec4<f32>(input.corner_radii);
    return output;
}

fn rounded_coverage(input: VertexOutput) -> f32 {
    let point = input.local_pixel;
    let size = input.pixel_size;
    let radii = input.corner_radii;
    var center = vec2<f32>(0.0, 0.0);
    var radius = 0.0;
    if point.x < radii.x && point.y < radii.x {
        center = vec2<f32>(radii.x, radii.x);
        radius = radii.x;
    } else if point.x > size.x - radii.y && point.y < radii.y {
        center = vec2<f32>(size.x - radii.y, radii.y);
        radius = radii.y;
    } else if point.x > size.x - radii.z && point.y > size.y - radii.z {
        center = vec2<f32>(size.x - radii.z, size.y - radii.z);
        radius = radii.z;
    } else if point.x < radii.w && point.y > size.y - radii.w {
        center = vec2<f32>(radii.w, size.y - radii.w);
        radius = radii.w;
    }
    if radius == 0.0 {
        return 1.0;
    }
    let distance_to_corner = distance(point, center);
    return 1.0 - smoothstep(radius - 0.5, radius + 0.5, distance_to_corner);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(atlas_texture, atlas_sampler, input.uv) * input.tint;
    return vec4<f32>(color.rgb, color.a * rounded_coverage(input));
}
