// VERTEX_FRAGMENT visibility
@group(2) @binding(1)
var<uniform> time: vec4<f32>; // Only the first f32 is used, must be padded to 16 bytes for web
@group(2) @binding(2)
var<uniform> screen_size: vec4<f32>; // Only vec2 is needed, but for web it must be padded to 16 bytes

struct Light {
  position: vec3<f32>,
  color: vec3<f32>,
}

@group(2) @binding(3)
var<uniform> light: Light;

struct CameraUniform {
  view_pos: vec4<f32>,
  view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;


// Vertex shader

// Displacement target is xyz and magnitude in the 4th position
@group(2) @binding(0)
var<uniform> displacement_target: vec4<f32>;

struct InstanceInput {
  @location(5) model_matrix_0: vec4<f32>,
  @location(6) model_matrix_1: vec4<f32>,
  @location(7) model_matrix_2: vec4<f32>,
  @location(8) model_matrix_3: vec4<f32>,
}

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
  @location(1) world_normal: vec3<f32>,
  @location(2) world_position: vec3<f32>,
  @location(3) @interpolate(perspective) screen_pos: vec2<f32>, // web cannot @interpolate(linear)
};

@vertex 
fn vs_main(
  model: VertexInput,
  instance: InstanceInput,
  @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
  let t = time[0];
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );
  var out: VertexOutput;
  out.tex_coords = model.tex_coords;
  let initial_world_position = model_matrix * vec4<f32>(model.position, 1.0);

  let displacement_strength = displacement_target.w;
  let diff = initial_world_position.xy - displacement_target.xy;
  let xy_displacement = displacement_strength * 3.0 * (-1.0 * pow(2.0, -1.0 * length(diff)) + 1.0) * normalize(diff);
  // Convenient variable for z_displacement and derivative_z_displacement
  // pow_component = e^(1.5 * length(diff) - 4)
  let exp_component = exp(1.5 * length(diff) - 4.0);
  let z_displacement_strength = displacement_strength / (1.0 + exp_component);
  // z_displacement_strength is a function that only outputs from 0 to 1, so invert that by subtracting from 1.
  let inverse_z_displacement_strength = 1.0 - z_displacement_strength;
  let z_displacement = 2.0 * z_displacement_strength;


  let displacement = vec4<f32>(xy_displacement, z_displacement, 0.0);

  // Transform the world position with sin/cos and time
  // Do this at an inverse rate to the z_displacement
  let wave_transform = vec4<f32>(
    0.3 * sin(t / 2.0 + initial_world_position.y / 2.0),
    0.3 * cos(t / 2.0 + initial_world_position.x / 2.0),
    0.2 * sin(t + initial_world_position.x + initial_world_position.y),
    0,
  ) * inverse_z_displacement_strength;

  let world_position = initial_world_position + displacement + wave_transform;
  out.world_position = world_position.xyz;

  out.clip_position = camera.view_proj * world_position;
  out.screen_pos = vec2<f32>(0.5, 0.5) * (out.clip_position.xy / out.clip_position.w + vec2<f32>(1.0, 1.0));

  // Calculate the normal. For now with my letters, every normal starts pointing straight up.
  let normal = vec3<f32>(0.0, 0.0, 1.0);
  // The normal is going to be perpendicular to the derivative of the z_displacement
  let derivative_z_displacement = (-4.5 * displacement_strength) * exp_component / pow(2.0, exp_component + 1);
  let derivative_wave = -1 * wave_transform.z; // The derivative is just *-1
  out.world_normal = normalize(normal - vec3<f32>(derivative_z_displacement * normalize(diff), 0.0) + derivative_wave * vec3<f32>(1.0, 1.0, 0.0));

  return out;
}


// Fragment shader
@group(0) @binding(0)
var t_letter: texture_2d<f32>;
@group(0) @binding(1)
var s_letter: sampler;

@fragment 
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  let object_color: vec4<f32> = textureSample(t_letter, s_letter, in.tex_coords);

  let ambient_strength = 0.01;
  let ambient_color = light.color * ambient_strength;

  let light_dir = normalize(light.position - in.world_position);

  let diffuse_strength = max(dot(in.world_normal, light_dir), 0.0);
  let diffuse_color = light.color * diffuse_strength;

  // Specular shading
  let view_dir = normalize(camera.view_pos.xyz - in.world_position);
  let half_dir = normalize(view_dir + light_dir);

  let specular_strength = pow(max(dot(in.world_normal, half_dir), 0.0), 32.0);
  let specular_color = specular_strength * light.color;

  //let result = specular_color;
  let result = (ambient_color + 0.5 * diffuse_color + 3.0 * specular_color) * object_color.xyz;
  return vec4<f32>(result, object_color.a);
  //return vec4<f32>(in.normal / 2.0 + vec3<f32>(0.5, 0.5, 0.5), 1.0); // This is a code snippet to check normal colors
}
