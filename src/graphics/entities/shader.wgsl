struct VertexInput {
    @location(0) position: vec3f,
    @location(1) normal: vec3f,
    @location(2) tex_coords: vec2f,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4f,
    @location(4) model_matrix_1: vec4f,
    @location(5) model_matrix_2: vec4f,
    @location(6) model_matrix_3: vec4f,

    @location(7) material_id: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) normal: vec3f,
    @location(1) tex_coords: vec2f,
    @location(2) position: vec3f,
    @location(3) material_id: u32,
};


@group(0) @binding(0)
var<uniform> view: mat4x4f;
@group(0) @binding(1)
var<uniform> proj: mat4x4f;

const INVALID_TEX_ID: u32 = 4294967295;

struct Material {
    diffuse_color: vec3f,

    diffuse_tex_id: u32,
}

@group(1) @binding(0)
var<storage, read> materials: array<Material>;

@group(2) @binding(0)
var t_atlas: texture_2d<f32>;
@group(2) @binding(1)
var s_atlas: sampler;

struct TextureAtlasUV {
    min: vec2f,
    max: vec2f,
}

@group(2) @binding(2)
var<storage, read> atlas_uvs: array<TextureAtlasUV>;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput
) -> VertexOutput {
    let model = mat4x4f(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let position = vec4f(vertex.position, 1.0);
    let mvp = proj * view * model;

    var out: VertexOutput;
    out.normal = vertex.normal;
    out.tex_coords = vertex.tex_coords;
    out.clip_position = mvp * position;
    out.position = position.xyz;
    out.material_id = instance.material_id;
    return out;
}


//TODO: impl
struct Light {
    position: vec3f,  // For point & spotlights
    intensity: f32,
    direction: vec3f, // For directional & spotlights
    cutoff: f32,          // Spotlight cutoff angle (cosine)
    color: vec3f, 
    light_type: u32,      // 0 = Point, 1 = Directional, 2 = Spotlight
};

@group(3) @binding(0)
var<storage, read> lights: array<Light>;
@group(3) @binding(1)
var<uniform> lights_count: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    let material = materials[in.material_id];
    let tex_id = material.diffuse_tex_id;
    var tex_color = vec4(1.0);
    if tex_id != INVALID_TEX_ID {
        let uvs = atlas_uvs[tex_id];
        tex_color = textureSample(t_atlas, s_atlas, lerp2(uvs.min, uvs.max, in.tex_coords));
    }

    var ambient = vec3f(0.2);
    for (var i: u32 = 0; i < lights_count; i = i + 1) {
        let light = lights[i];
        var light_dir = normalize(light.position - in.position);
        let light_dist = length(light.position.xyz - in.position.xyz);
        let attenuation = 1.0 / (1.0 + 0.09 * light_dist + 0.032 * light_dist * light_dist);
        
        if light.light_type == 1 {
            ambient += diffuse(in.normal, light_dir) * attenuation * light.intensity * light.color;
        }else if light.light_type == 2 { 
            ambient += diffuse(in.normal, -light.direction) * light.intensity * light.color;
        }else if light.light_type == 3 {
            let spot_effect = dot(light_dir, light.direction); // Cosine of angle

            if spot_effect > light.cutoff { 
                let intensity = smoothstep(light.cutoff, light.cutoff + 0.1, spot_effect);
                ambient += diffuse(in.normal, light_dir) * intensity * attenuation * light.intensity * light.color;
            }
        }
    }
    
    let diffuse_color = tex_color * vec4(material.diffuse_color, 1.);
    return diffuse_color * vec4(ambient, 1.);
}

fn diffuse(normal: vec3f, light_dir: vec3f) -> f32 { return max(dot(normal, light_dir), 0.0); }

fn lerp2(a: vec2f, b: vec2f, t: vec2f) -> vec2f { return a + (b - a) * t; }