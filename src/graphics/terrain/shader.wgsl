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
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@group(0) @binding(0)
var<uniform> inv_view: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> inv_proj: mat4x4<f32>;



const MAX_STEPS: u32 = 128;
const EPS: f32 = 0.01;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> FragOutput {
    let resolution = vec2<f32>(800.0, 600.0);
    let uv = (frag_coord.xy / resolution);
    let ndc = vec2(uv.x, 1. - uv.y) * 2.0 - 1.0;

    let near = 1./(inv_proj[3][3] - inv_proj[2][3]);
    let far = 1./(inv_proj[3][3] + inv_proj[2][3]);

    let clip_dir = vec4<f32>(ndc, -1.0, 1.0);
    let view_dir = normalize(pdiv(inv_proj * clip_dir));
    let ray_dir = normalize((inv_view * vec4<f32>(view_dir, 0.)).xyz);
    let ray_origin = vec3<f32>(inv_view[3][0], inv_view[3][1], inv_view[3][2]);

    var out: FragOutput;
    out.depth = 1.;

    var t = 0.;
    var first = true;
    for (var i = 0u; i < MAX_STEPS; i++) {
        let p = ray_origin + t * ray_dir;
        let d = sdf_torus(p, 3., .5); 
        
        if (d < EPS) {
            let world_pos = t * view_dir;

            out.color = vec4<f32>((p+2.)/4., 1.0); // temp color
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


fn pdiv(v: vec4<f32>) -> vec3<f32> {
    return v.xyz / v.w;
}

fn sdf_sphere(p: vec3<f32>, radius: f32) -> f32 {
    return length(p) - radius;
}

fn sdf_torus(p: vec3<f32>, R: f32, r: f32) -> f32 {
    let q = vec2<f32>(length(p.xz) - R, p.y);
    return length(q) - r;
}