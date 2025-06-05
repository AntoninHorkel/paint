struct VertexUniformBufferObject {
    scale: vec2<f32>,
    offset: vec2<f32>,
}
@group(0) @binding(0) var<uniform> vu: VertexUniformBufferObject;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(@location(0) position: vec2<f32>) -> VertexOutput {
    return VertexOutput(
        fma(position, vec2<f32>(0.5, -0.5), vec2<f32>(0.5)),
        vec4<f32>((position + vu.offset) * vu.scale, 0.0, 1.0),
    );
}

struct StorageBufferObject {
    length: u32,
    points: array<vec2<f32>>,
}
@group(0) @binding(1) var<storage, read> s: StorageBufferObject;
struct FragmentUniformBufferObject {
    grid_scale: vec2<f32>,
    action: u32,
    preview: u32, // bool
}
@group(0) @binding(2) var<uniform> fu: FragmentUniformBufferObject;
@group(0) @binding(3) var texture: texture_2d<f32>;
@group(0) @binding(4) var texture_sampler: sampler;

@fragment
fn fragment(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let color = textureSample(texture, texture_sampler, uv);

    if s.length < 2 {
        return color;
    }

    // TODO: Pass this as an uniform?
    let frag_coord = uv * vec2<f32>(textureDimensions(texture));

    let p1 = s.points[0];
    let p2 = s.points[1];

    var sdf = 1e6;
    switch fu.action {
        case 1u, 2u, 3u, 4u, 8u: {
            sdf = min(distance(p1, frag_coord), distance(p2, frag_coord)) - 5.0;
        }
        case 5u: {
            for (var i = 0u; i < s.length; i += 1) {
                sdf = min(sdf, distance(s.points[i], frag_coord));
            }
            sdf -= 5.0;
        }
        default: {
            return color;
        }
    }
    if !bool(fu.preview) {
        switch fu.action {
            // Draw line
            case 1u: {
                // https://iquilezles.org/articles/distfunctions2d/
                // https://www.youtube.com/watch?v=PMltMdi1Wzg
                // https://www.desmos.com/calculator/afsee2587r
                let a = frag_coord - p1;
                let b = p2 - p1;
                sdf = min(sdf, distance(a, b * clamp(dot(a, b) / dot(b, b), 0.0, 1.0)));
            }
            // Draw or cut rectangle
            case 2u, 8u: {
                // https://math.stackexchange.com/a/69134
                // https://www.desmos.com/calculator/wekvvsxdof
                // let a = abs((frag_coord * 2.0 - p1 - p2) / (p1 - p2)); // = abs((frag_coord - (p1 + p2) / 2.0) / (p1 - p2) / 2.0)
                // sdf = fwidthFine(min(sdf, max(a.x, a.y) - 1.0));
                // https://iquilezles.org/articles/distfunctions2d/
                // https://www.youtube.com/watch?v=62-pRVZuS5c
                let p_min = min(p1, p2);
                let p_max = max(p1, p2);
                let a = abs(frag_coord - (p_min + p_max) / 2.0) - (p_min - p_max) / 2.0;
                sdf = min(sdf, length(max(a, vec2<f32>(0.0))) + min(max(a.x, a.y), 0.0));
            }
            // Draw circle
            case 3u: {
                // https://iquilezles.org/articles/distfunctions2d/
                let a = distance(p1, frag_coord) - distance(p1, p2);
                sdf = max(min(sdf, a), -max(sdf, a)); // Simple OR doesn't work for circles, XOR is used: https://iquilezles.org/articles/sdfxor/
            }
            // Draw ellipse
            case 4u: {}
            // Draw polygon
            case 5u: {
                // https://iquilezles.org/articles/distfunctions2d/
                var d = dot(frag_coord - s.points[0], frag_coord - s.points[0]);
                var i = 0u;
                var j = s.length - 1;
                for (; i < s.length;) {
                    let a = frag_coord - s.points[i];
                    let b = s.points[j] - s.points[i];
                    let c = a - b * clamp(dot(a, b) / dot(b, b), 0.0, 1.0);
                    d = min(d, dot(c, c));
                    j = i;
                    i += 1;
                }
                sdf = min(sdf, sqrt(d));
            }
            default: {
                return color;
            }
        }
    }

    let stroke = 1.0;
    return vec4<f32>(mix(
        color.rgb,
        // Luma coefficients for grayscale:
        // https://en.wikipedia.org/wiki/Rec._709#Luma_coefficients
        // https://en.wikipedia.org/wiki/SRGB#Primaries
        // WCAG contrast ratio: 0.179
        select(vec3<f32>(0.0), vec3<f32>(1.0), dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722)) < 0.179),
        1.0 - smoothstep(stroke * 0.9, stroke * 1.1, abs(sdf)),
    ), 1.0);
}
