struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tex_coords: vec2<f32>,
};

struct InstanceInput {
    @location(3) model_matrix_0: vec4<f32>,
    @location(4) model_matrix_1: vec4<f32>,
    @location(5) model_matrix_2: vec4<f32>,
    @location(6) model_matrix_3: vec4<f32>,

    @location(7) material_id: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) position: vec3<f32>,
    @location(3) material_id: u32,
};


@group(0) @binding(0)
var<uniform> view: mat4x4<f32>;
@group(0) @binding(1)
var<uniform> proj: mat4x4<f32>;

const INVALID_TEX_ID: u32 = 4294967295;

struct Material {
    diffuse_color: vec3<f32>,

    diffuse_tex_id: u32,
}

@group(1) @binding(0)
var<storage, read> materials: array<Material>;

@group(2) @binding(0)
var t_atlas: texture_2d<f32>;
@group(2) @binding(1)
var s_atlas: sampler;

struct TextureAtlasUV {
    min: vec2<f32>,
    max: vec2<f32>,
}

@group(2) @binding(2)
var<storage, read> atlas_uvs: array<TextureAtlasUV>;

@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput
) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let position = model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.normal = vertex.normal;
    out.tex_coords = vertex.tex_coords;
    out.clip_position = proj * view * position;
    out.position = position.xyz;
    out.material_id = instance.material_id;
    return out;
}


//TODO: impl
struct Light {
    position: vec3<f32>,  // For point & spotlights
    intensity: f32,
    direction: vec3<f32>, // For directional & spotlights
    cutoff: f32,          // Spotlight cutoff angle (cosine)
    color: vec3<f32>, 
    light_type: u32,      // 0 = Point, 1 = Directional, 2 = Spotlight
};

@group(3) @binding(0)
var<storage, read> lights: array<Light>;
@group(3) @binding(1)
var<uniform> lights_count: u32;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let material = materials[in.material_id];
    let tex_id = material.diffuse_tex_id;
    var tex_color = vec4(1.0);
    if tex_id != INVALID_TEX_ID {
        let uvs = atlas_uvs[tex_id];
        tex_color = textureSample(t_atlas, s_atlas, lerp2(uvs.min, uvs.max, in.tex_coords));
    }

    var ambient = vec3<f32>(0.2);
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

fn diffuse(normal: vec3<f32>, light_dir: vec3<f32>) -> f32 { return max(dot(normal, light_dir), 0.0); }

fn lerp2(a: vec2<f32>, b: vec2<f32>, t: vec2<f32>) -> vec2<f32> { return a + (b - a) * t; }