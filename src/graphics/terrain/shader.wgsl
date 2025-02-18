@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    var positions = array<vec2f, 6>(
        vec2f(-1.0, -1.0),
        vec2f( 1.0, -1.0),
        vec2f(-1.0,  1.0),
        vec2f(-1.0,  1.0),
        vec2f( 1.0, -1.0),
        vec2f( 1.0,  1.0)
    );
    
    return vec4f(positions[vertex_index], 0.0, 1.0);
}

struct FragOutput {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
};

@group(0) @binding(0)
var<uniform> inv_view: mat4x4f;
@group(0) @binding(1)
var<uniform> inv_proj: mat4x4f;
@group(0) @binding(2)
var<uniform> viewport_size: vec2<u32>;



const MAX_STEPS: u32 = 128;
const EPS: f32 = 0.01;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4f) -> FragOutput {
    let uv = (frag_coord.xy / vec2f(viewport_size));
    let ndc = vec2(uv.x, 1. - uv.y) * 2.0 - 1.0;

    let near = 1./(inv_proj[3][3] - inv_proj[2][3]);
    let far = 1./(inv_proj[3][3] + inv_proj[2][3]);

    let clip_dir = vec4f(ndc, -1.0, 1.0);
    let view_dir = normalize(pdiv(inv_proj * clip_dir)); // issue when changing aspect_ratio
    let ray_dir = normalize((inv_view * vec4f(view_dir, 0.)).xyz);
    let ray_origin = vec3f(inv_view[3][0], inv_view[3][1], inv_view[3][2]);

    var out: FragOutput;
    out.depth = 1.;

    var t = 0.;
    var first = true;
    for (var i = 0u; i < MAX_STEPS; i++) {
        let p = ray_origin + t * ray_dir;
        let d = sdf_torus(p, 3., .5); 
        
        if (d < EPS) {
            let world_pos = t * view_dir;

            out.color = vec4f((p+2.)/4., 1.0); // temp color
            let depth = (far+near)/(far-near) + 2.*far*near/(far-near) / world_pos.z;
            out.depth = select(depth, 0.0, first);

            break;
        }
        
        if (t > far) {
            break;
        }
        
        t += d;
        first = false;
    }

    return out;
}


fn pdiv(v: vec4f) -> vec3f {
    return v.xyz / v.w;
}

fn sdf_sphere(p: vec3f, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sdf_torus(p: vec3f, R: f32, r: f32) -> f32 {
    let q = vec2f(length(p.xz) - R, p.y);
    return length(q) - r;
}