const PI: f32 = 3.141592653589793;

struct CameraLighting {
    view_projection: mat4x4<f32>,
    light_directions: array<vec4<f32>, 5>,
    light_colors: array<vec4<f32>, 5>,
    base_color: vec4<f32>,
    camera_position: vec4<f32>,
    ambient: vec4<f32>,
};

struct MaterialParameters {
    base_color_factor: vec4<f32>,
    emissive_alpha_cutoff: vec4<f32>,
    metallic_roughness_normal_occlusion: vec4<f32>,
    tex_coord_sets: vec4<f32>,
    occlusion_alpha_mode: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: CameraLighting;

@group(1) @binding(0)
var<uniform> material: MaterialParameters;
@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_color_sampler: sampler;
@group(1) @binding(3)
var metallic_roughness_texture: texture_2d<f32>;
@group(1) @binding(4)
var metallic_roughness_sampler: sampler;
@group(1) @binding(5)
var normal_texture: texture_2d<f32>;
@group(1) @binding(6)
var normal_sampler: sampler;
@group(1) @binding(7)
var emissive_texture: texture_2d<f32>;
@group(1) @binding(8)
var emissive_sampler: sampler;
@group(1) @binding(9)
var occlusion_texture: texture_2d<f32>;
@group(1) @binding(10)
var occlusion_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(9) tex_coord_0: vec2<f32>,
    @location(10) tex_coord_1: vec2<f32>,
    @location(11) color: vec4<f32>,
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) normal_0: vec4<f32>,
    @location(7) normal_1: vec4<f32>,
    @location(8) normal_2: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) tex_coord_0: vec2<f32>,
    @location(3) tex_coord_1: vec2<f32>,
    @location(4) color: vec4<f32>,
};

@vertex
fn vertex_main(input: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(
        input.model_0,
        input.model_1,
        input.model_2,
        input.model_3,
    );
    let normal_matrix = mat3x3<f32>(
        input.normal_0.xyz,
        input.normal_1.xyz,
        input.normal_2.xyz,
    );
    let world_position = model * vec4<f32>(input.position, 1.0);

    var output: VertexOutput;
    output.clip_position = globals.view_projection * world_position;
    output.world_position = world_position.xyz;
    output.world_normal = normalize(normal_matrix * input.normal);
    output.tex_coord_0 = input.tex_coord_0;
    output.tex_coord_1 = input.tex_coord_1;
    output.color = input.color;
    return output;
}

fn selected_uv(set_index: f32, uv0: vec2<f32>, uv1: vec2<f32>) -> vec2<f32> {
    return select(uv0, uv1, set_index > 0.5);
}

fn distribution_ggx(normal: vec3<f32>, half_vector: vec3<f32>, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let n_dot_h = max(dot(normal, half_vector), 0.0);
    let denominator = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * denominator * denominator, 0.0001);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = r * r / 8.0;
    return n_dot_v / max(n_dot_v * (1.0 - k) + k, 0.0001);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

fn mapped_normal(input: VertexOutput, front_facing: bool) -> vec3<f32> {
    var geometric = normalize(input.world_normal);
    if (!front_facing) {
        geometric = -geometric;
    }
    let uv = selected_uv(
        material.tex_coord_sets.z,
        input.tex_coord_0,
        input.tex_coord_1,
    );
    let dpdx_position = dpdx(input.world_position);
    let dpdy_position = dpdy(input.world_position);
    let dpdx_uv = dpdx(uv);
    let dpdy_uv = dpdy(uv);
    let determinant = dpdx_uv.x * dpdy_uv.y - dpdx_uv.y * dpdy_uv.x;
    if (abs(determinant) < 0.000001) {
        return geometric;
    }
    let tangent = normalize((dpdx_position * dpdy_uv.y - dpdy_position * dpdx_uv.y) / determinant);
    let bitangent = normalize((-dpdx_position * dpdy_uv.x + dpdy_position * dpdx_uv.x) / determinant);
    var sampled = textureSample(normal_texture, normal_sampler, uv).xyz * 2.0 - 1.0;
    sampled = vec3<f32>(
        sampled.xy * material.metallic_roughness_normal_occlusion.z,
        sampled.z,
    );
    return normalize(mat3x3<f32>(tangent, bitangent, geometric) * sampled);
}

@fragment
fn fragment_main(input: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let base_uv = selected_uv(material.tex_coord_sets.x, input.tex_coord_0, input.tex_coord_1);
    var base = textureSample(base_color_texture, base_color_sampler, base_uv)
        * material.base_color_factor
        * input.color;
    let alpha_mode = material.occlusion_alpha_mode.y;
    if (alpha_mode > 0.5 && alpha_mode < 1.5 && base.a < material.emissive_alpha_cutoff.w) {
        discard;
    }
    if (alpha_mode < 1.5) {
        base.a = 1.0;
    }
    if (material.occlusion_alpha_mode.z > 0.5) {
        return base;
    }

    let mr_uv = selected_uv(material.tex_coord_sets.y, input.tex_coord_0, input.tex_coord_1);
    let mr = textureSample(metallic_roughness_texture, metallic_roughness_sampler, mr_uv);
    let metallic = clamp(mr.b * material.metallic_roughness_normal_occlusion.x, 0.0, 1.0);
    let roughness = clamp(mr.g * material.metallic_roughness_normal_occlusion.y, 0.045, 1.0);
    let normal = mapped_normal(input, front_facing);
    let toward_camera = normalize(globals.camera_position.xyz - input.world_position);
    let n_dot_v = max(dot(normal, toward_camera), 0.0);

    let f0 = mix(vec3<f32>(0.04), base.rgb, metallic);
    var direct = vec3<f32>(0.0);
    for (var index = 0u; index < 5u; index += 1u) {
        if (globals.light_colors[index].w <= 0.0) {
            continue;
        }
        let toward_light = normalize(-globals.light_directions[index].xyz);
        let half_vector = normalize(toward_camera + toward_light);
        let n_dot_l = max(dot(normal, toward_light), 0.0);
        let h_dot_v = max(dot(half_vector, toward_camera), 0.0);
        let fresnel = fresnel_schlick(h_dot_v, f0);
        let distribution = distribution_ggx(normal, half_vector, roughness);
        let geometry = geometry_schlick_ggx(n_dot_v, roughness)
            * geometry_schlick_ggx(n_dot_l, roughness);
        let specular = distribution * geometry * fresnel / max(4.0 * n_dot_v * n_dot_l, 0.0001);
        let diffuse_weight = (vec3<f32>(1.0) - fresnel) * (1.0 - metallic);
        direct += (diffuse_weight * base.rgb / PI + specular)
            * globals.light_colors[index].rgb
            * globals.light_colors[index].w
            * n_dot_l;
    }

    let occlusion_uv = selected_uv(
        material.occlusion_alpha_mode.x,
        input.tex_coord_0,
        input.tex_coord_1,
    );
    let sampled_occlusion = textureSample(occlusion_texture, occlusion_sampler, occlusion_uv).r;
    let occlusion = mix(1.0, sampled_occlusion, material.metallic_roughness_normal_occlusion.w);
    let ambient = base.rgb * globals.ambient.x * occlusion;
    let emissive_uv = selected_uv(material.tex_coord_sets.w, input.tex_coord_0, input.tex_coord_1);
    let emissive = textureSample(emissive_texture, emissive_sampler, emissive_uv).rgb
        * material.emissive_alpha_cutoff.rgb;
    return vec4<f32>(direct + ambient + emissive, base.a);
}
