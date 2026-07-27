struct CameraLighting {
    view_projection: mat4x4<f32>,
    light_directions: array<vec4<f32>, 5>,
    light_colors: array<vec4<f32>, 5>,
    base_color: vec4<f32>,
    camera_position: vec4<f32>,
    ambient: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: CameraLighting;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
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
    @location(0) world_normal: vec3<f32>,
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
    output.world_normal = normalize(normal_matrix * input.normal);
    return output;
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.world_normal);
    var lighting = vec3<f32>(globals.ambient.x);
    for (var index = 0u; index < 5u; index += 1u) {
        if (globals.light_colors[index].w <= 0.0) {
            continue;
        }
        let toward_light = normalize(-globals.light_directions[index].xyz);
        let diffuse = max(dot(normal, toward_light), 0.0);
        lighting += diffuse * globals.light_colors[index].w * globals.light_colors[index].rgb;
    }
    let color = globals.base_color.rgb * lighting;
    return vec4<f32>(color, globals.base_color.a);
}
