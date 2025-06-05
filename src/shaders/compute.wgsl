struct StorageBufferObject {
    length: u32,
    points: array<vec2<f32>>,
}
@group(0) @binding(0) var<storage, read> s: StorageBufferObject;
struct UniformBufferObject {
    color: vec4<f32>,
    action: u32,
    stroke: f32,
    anti_aliasing_scale: f32,
    dash_length: f32,
    gap_length: f32,
};
@group(0) @binding(1) var<uniform> u: UniformBufferObject;
@group(0) @binding(2) var texture: texture_storage_2d<rgba8unorm, read_write>;

@compute @workgroup_size(8, 8, 1)
fn compute(@builtin(global_invocation_id) id: vec3<u32>) {
    if any(id.xy >= textureDimensions(texture)) || (s.length < 2 && u.action != 0) {
        return;
    }

    let p1 = s.points[0];
    let p2 = s.points[1];
    let current_pixel = vec2<f32>(id.xy);

    var sdf = 1e6;
    switch u.action {
        // Init
        case 0u: {
            textureStore(texture, id.xy, u.color);
            return;
        }
        // Draw line
        case 1u: {
            // https://iquilezles.org/articles/distfunctions2d/
            // https://www.youtube.com/watch?v=PMltMdi1Wzg
            // https://www.desmos.com/calculator/afsee2587r
            let a = current_pixel - p1;
            let b = p2 - p1;
            let t = clamp(dot(a, b) / dot(b, b), 0.0, 1.0);
            sdf = distance(a, b * t);
            let period = u.dash_length + u.gap_length;
            if period > 0.0 {
                let d = t * length(b);
                if d % period >= u.dash_length {
                    sdf = 1e6;
                }
            }
        }
        // Draw rectangle
        case 2u: {
            // https://iquilezles.org/articles/distfunctions2d/
            // https://www.youtube.com/watch?v=62-pRVZuS5c
            let p_min = min(p1, p2);
            let p_max = max(p1, p2);
            let a = abs(current_pixel - (p_min + p_max) / 2.0) - (p_max - p_min) / 2.0;
            sdf = length(max(a, vec2<f32>(0.0))) + min(max(a.x, a.y), 0.0);
        }
        // Draw circle
        case 3u: {
            // https://iquilezles.org/articles/distfunctions2d/
            sdf = distance(p1, current_pixel) - distance(p1, p2);
        }
        // Draw ellipse
        case 4u: {}
        // Draw polygon
        case 5u: {
            // https://iquilezles.org/articles/distfunctions2d/
            var d = dot(current_pixel - s.points[0], current_pixel - s.points[0]);
            var i = 0u;
            var j = s.length - 1;
            for (; i < s.length;) {
                let a = current_pixel - s.points[i];
                let b = s.points[j] - s.points[i];
                let c = a - b * clamp(dot(a, b) / dot(b, b), 0.0, 1.0);
                d = min(d, dot(c, c));
                j = i;
                i += 1;
            }
            sdf = sqrt(d);
        }
        // Erase
        case 6u: {
            // https://iquilezles.org/articles/distfunctions2d/
            // https://www.youtube.com/watch?v=PMltMdi1Wzg
            // https://www.desmos.com/calculator/afsee2587r
            let a = current_pixel - p1;
            let b = p2 - p1;
            sdf = distance(a, b * clamp(dot(a, b) / dot(b, b), 0.0, 1.0));
        }
        default: {
            return;
        }
    }

    sdf = abs(sdf);
    var coverage = 0.0;
    if u.anti_aliasing_scale > 0.0 {
        coverage = 1.0 - smoothstep(
            u.stroke * (1.0 - u.anti_aliasing_scale),
            u.stroke * (1.0 + u.anti_aliasing_scale),
            sdf,
        );
    } else {
        coverage = f32(sdf <= u.stroke);
    }
    let blend_alpha = coverage * u.color.a;
    if blend_alpha > 0.0 {
        textureStore(texture, id.xy, mix(textureLoad(texture, id.xy), u.color, blend_alpha));
    }
}
