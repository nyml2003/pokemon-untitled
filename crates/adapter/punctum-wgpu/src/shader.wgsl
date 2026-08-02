struct ViewportUniform {
    target_size: vec2<u32>,
    origin: vec2<i32>,
    cell_size: vec2<u32>,
    atlas_size: vec2<u32>,
}

struct RadarData {
    values: array<u32, 6>,
    max: u32,
    rings: u32,
    grid_color: vec4<u32>,
    axis_color: vec4<u32>,
    fill_color: vec4<u32>,
    edge_color: vec4<u32>,
    point_color: vec4<u32>,
    bounds: vec4<u32>,
}

@group(0) @binding(0)
var<uniform> viewport: ViewportUniform;

@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

@group(0) @binding(3)
var<storage, read> radar_data: array<RadarData>;

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
    @interpolate(flat) @location(5) primitive: u32,
    @interpolate(flat) @location(6) instance_index: u32,
    @location(7) global_pixel: vec2<f32>,
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
fn vs_main(
    input: VertexInput,
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
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
    output.tint = input.tint;
    output.local_pixel = corner * vec2<f32>(input.grid_span * viewport.cell_size);
    output.pixel_size = vec2<f32>(input.grid_span * viewport.cell_size);
    output.corner_radii = vec4<f32>(input.corner_radii);
    output.primitive = input.visible;
    output.instance_index = instance_index;
    output.global_pixel = pixel;
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

fn radar_color(color: vec4<u32>) -> vec4<f32> {
    return vec4<f32>(color) / 255.0;
}

fn radar_value(data: RadarData, index: u32) -> f32 {
    if data.max == 0u {
        return 0.0;
    }
    return f32(data.values[index]) / f32(data.max);
}

fn radar_position(center: vec2<f32>, radius: f32, scale: f32, index: u32) -> vec2<f32> {
    let angle = -1.5707963 + f32(index) * 1.0471976;
    return center + vec2<f32>(cos(angle), sin(angle)) * radius * scale;
}

fn radar_polygon_inside(point: vec2<f32>, points: array<vec2<f32>, 6>) -> bool {
    var inside = false;
    for (var index = 0u; index < 6u; index = index + 1u) {
        let next = (index + 1u) % 6u;
        let first = points[index];
        let second = points[next];
        if ((first.y > point.y) != (second.y > point.y)) {
            let crossing = (second.x - first.x) * (point.y - first.y)
                / (second.y - first.y) + first.x;
            if point.x < crossing {
                inside = !inside;
            }
        }
    }
    return inside;
}

fn radar_segment_distance(point: vec2<f32>, first: vec2<f32>, second: vec2<f32>) -> f32 {
    let direction = second - first;
    let length_squared = max(dot(direction, direction), 0.0001);
    let projection = clamp(dot(point - first, direction) / length_squared, 0.0, 1.0);
    return distance(point, first + direction * projection);
}

fn radar_lines_hit(
    point: vec2<f32>,
    lines: array<vec2<f32>, 6>,
    threshold: f32,
) -> bool {
    for (var index = 0u; index < 6u; index = index + 1u) {
        let next = (index + 1u) % 6u;
        if radar_segment_distance(point, lines[index], lines[next]) <= threshold {
            return true;
        }
    }
    return false;
}

fn radar_fragment(input: VertexOutput) -> vec4<f32> {
    let data = radar_data[input.instance_index];
    let chart_origin = vec2<f32>(data.bounds.xy);
    let chart_size = vec2<f32>(data.bounds.zw);
    let center = chart_origin + chart_size * 0.5;
    let radius = max(min(chart_size.x, chart_size.y) * 0.5 - 20.0, 1.0);
    var outer: array<vec2<f32>, 6>;
    var values: array<vec2<f32>, 6>;
    for (var index = 0u; index < 6u; index = index + 1u) {
        outer[index] = radar_position(center, radius, 1.0, index);
        values[index] = radar_position(center, radius, radar_value(data, index), index);
    }

    var color = vec4<f32>(0.0);
    if data.fill_color.a > 0u && radar_polygon_inside(input.global_pixel, values) {
        color = radar_color(data.fill_color);
    }

    let ring_count = max(data.rings, 1u);
    for (var ring = 1u; ring <= 8u; ring = ring + 1u) {
        if ring <= ring_count {
            var ring_points: array<vec2<f32>, 6>;
            ring_points[0] = radar_position(center, radius, f32(ring) / f32(ring_count), 0u);
            ring_points[1] = radar_position(center, radius, f32(ring) / f32(ring_count), 1u);
            ring_points[2] = radar_position(center, radius, f32(ring) / f32(ring_count), 2u);
            ring_points[3] = radar_position(center, radius, f32(ring) / f32(ring_count), 3u);
            ring_points[4] = radar_position(center, radius, f32(ring) / f32(ring_count), 4u);
            ring_points[5] = radar_position(center, radius, f32(ring) / f32(ring_count), 5u);
            if radar_lines_hit(input.global_pixel, ring_points, 0.75) {
                color = radar_color(data.grid_color);
            }
        }
    }

    for (var index = 0u; index < 6u; index = index + 1u) {
        if radar_segment_distance(input.global_pixel, center, outer[index]) <= 0.75 {
            color = radar_color(data.axis_color);
        }
    }
    if radar_lines_hit(input.global_pixel, values, 1.0) {
        color = radar_color(data.edge_color);
    }
    for (var index = 0u; index < 6u; index = index + 1u) {
        if distance(input.global_pixel, values[index]) <= 2.5 {
            color = radar_color(data.point_color);
        }
    }
    return color;
}

/// 确定性坐标哈希，用于天气粒子的伪随机分布。
fn hash_pixel(coord: vec2<u32>) -> f32 {
    let value = sin(f32(coord.x) * 127.1 + f32(coord.y) * 311.7) * 43758.5453;
    return value - floor(value);
}

/// 雨：近/远两层斜雨丝带尾迹渐变，整体蓝灰氛围。
fn weather_rain(input: VertexOutput, frame: u32, tint: vec4<f32>) -> vec4<f32> {
    let size = input.pixel_size;
    let point = input.local_pixel;
    var result = vec4<f32>(tint.rgb, tint.a * 0.35);
    result = result + rain_streak(point, size, f32(frame) * 0.6, 9.0, 7.0, 1.4, 0.5, tint, 1u);
    result = result + rain_streak(point, size, f32(frame) * 1.8, 22.0, 13.0, 3.2, 0.3, tint, 2u);
    return result;
}

/// 单层雨丝：一列内一条斜向下落、下端实上端淡的尾迹。
fn rain_streak(
    point: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    column_width: f32,
    length: f32,
    slope: f32,
    density: f32,
    tint: vec4<f32>,
    seed: u32,
) -> vec4<f32> {
    let column = u32(point.x / column_width);
    let random = hash_pixel(vec2<u32>(column, seed));
    if random >= density {
        return vec4<f32>(0.0);
    }
    let cycle = size.y + length + 8.0;
    let travel = (time + random * cycle) % cycle;
    let top = travel - length;
    if point.y < top || point.y > top + length {
        return vec4<f32>(0.0);
    }
    let along = (point.y - top) / length;
    let line_x = f32(column) * column_width + random * (column_width - 4.0) + along * slope;
    let dist = abs(point.x - line_x);
    let core = 1.0 - smoothstep(0.3, 1.2, dist);
    let fade = smoothstep(0.0, 0.45, along);
    return vec4<f32>(tint.rgb, tint.a * core * fade);
}

/// 沙暴：近/远两层沙粒横向流动，底部黄褐尘雾。
fn weather_sandstorm(input: VertexOutput, frame: u32, tint: vec4<f32>) -> vec4<f32> {
    let size = input.pixel_size;
    let point = input.local_pixel;
    var result = vec4<f32>(tint.rgb, tint.a * 0.4);
    result = result + sand_layer(point, size, f32(frame), 8.0, 0.55, 0.9, tint, 11u);
    result = result + sand_layer(point, size, f32(frame) * 2.2, 18.0, 0.3, 1.8, tint, 13u);
    let dust = smoothstep(size.y * 0.35, size.y, point.y);
    result = result + vec4<f32>(tint.rgb, tint.a * 0.3 * dust);
    return result;
}

/// 单层沙粒：一行内一个沙粒从右向左流过。
fn sand_layer(
    point: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    row_height: f32,
    density: f32,
    grain_size: f32,
    tint: vec4<f32>,
    seed: u32,
) -> vec4<f32> {
    let row = u32(point.y / row_height);
    let random = hash_pixel(vec2<u32>(row, seed));
    if random >= density {
        return vec4<f32>(0.0);
    }
    let cycle = size.x + 16.0;
    let travel = (time + random * cycle) % cycle;
    let grain_x = travel - 8.0;
    let grain_y = f32(row) * row_height + random * (row_height - 2.0);
    let dist = abs(point.x - grain_x) + abs(point.y - grain_y);
    let coverage = 1.0 - smoothstep(grain_size, grain_size * 2.4, dist);
    return vec4<f32>(tint.rgb, tint.a * coverage);
}

/// 晴天：暖色光晕 + 顶部丁达尔光柱 + 轻微热浪。
fn weather_sun(input: VertexOutput, frame: u32, tint: vec4<f32>) -> vec4<f32> {
    let size = input.pixel_size;
    let point = input.local_pixel;
    let center = size * 0.5;
    let dist = abs(point.x - center.x) + abs(point.y - center.y);
    let max_dist = size.x * 0.5 + size.y * 0.5;
    let halo = 1.0 - dist / max_dist;
    // 丁达尔光柱：横向几道暖色光柱，随帧轻微移动
    let shaft = 0.5 + 0.5 * sin(point.x * 0.02 + sin(f32(frame) * 0.02) * 0.5);
    let beam = pow(shaft, 7.0);
    // 热浪：轻微亮度扰动
    let heat = 0.9 + 0.1 * sin((point.x + point.y) * 0.01 + f32(frame) * 0.04);
    let strength = (0.4 + 0.6 * halo) * (0.85 + 0.15 * beam) * heat;
    return vec4<f32>(tint.rgb, tint.a * strength);
}

/// 冰雹：近/远两层冰粒下落，带横向摆动。
fn weather_hail(input: VertexOutput, frame: u32, tint: vec4<f32>) -> vec4<f32> {
    let size = input.pixel_size;
    let point = input.local_pixel;
    var result = vec4<f32>(tint.rgb, tint.a * 0.35);
    result = result + hail_layer(point, size, f32(frame) * 0.8, 8.0, 0.4, 1.0, tint, 3u);
    result = result + hail_layer(point, size, f32(frame) * 1.6, 18.0, 0.18, 2.0, tint, 7u);
    return result;
}

/// 单层冰粒：每列一个冰粒下落，带横向 sin 摆动。
fn hail_layer(
    point: vec2<f32>,
    size: vec2<f32>,
    time: f32,
    column_width: f32,
    density: f32,
    grain_size: f32,
    tint: vec4<f32>,
    seed: u32,
) -> vec4<f32> {
    let column = u32(point.x / column_width);
    let random = hash_pixel(vec2<u32>(column, seed));
    if random >= density {
        return vec4<f32>(0.0);
    }
    let cycle = size.y + 16.0;
    let travel = (time + random * cycle) % cycle;
    let grain_y = travel - 8.0;
    let sway = sin(time * 0.1 + random * 6.2831) * 3.0;
    let grain_x = f32(column) * column_width + random * (column_width - 4.0) + sway;
    let dist = abs(point.x - grain_x) + abs(point.y - grain_y);
    let coverage = 1.0 - smoothstep(grain_size, grain_size * 2.6, dist);
    return vec4<f32>(tint.rgb, tint.a * coverage);
}

fn weather_fragment(input: VertexOutput) -> vec4<f32> {
    let pattern = u32(input.corner_radii.x);
    let frame = u32(input.corner_radii.y);
    let tint = input.tint;
    if pattern == 0u {
        return weather_rain(input, frame, tint);
    }
    if pattern == 1u {
        return weather_sandstorm(input, frame, tint);
    }
    if pattern == 2u {
        return weather_sun(input, frame, tint);
    }
    return weather_hail(input, frame, tint);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if input.primitive == 3u {
        return radar_fragment(input);
    }
    if input.primitive == 4u {
        return weather_fragment(input);
    }

    let color = textureSample(atlas_texture, atlas_sampler, input.uv) * input.tint;
    var coverage = rounded_coverage(input);
    if input.primitive == 2u {
        let center = input.corner_radii.xy;
        let radius = input.corner_radii.z;
        coverage = 1.0 - smoothstep(
            radius - 0.5,
            radius + 0.5,
            distance(input.local_pixel, center),
        );
    }
    return vec4<f32>(color.rgb, color.a * coverage);
}

