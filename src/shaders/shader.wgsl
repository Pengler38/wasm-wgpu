// VERTEX_FRAGMENT visibility
@group(2) @binding(1)
var<uniform> time: f32;
@group(2) @binding(2)
var<uniform> screen_size: vec4<f32>; // Only vec2 is needed, but for web it must be padded to 16 bytes


// Vertex shader
struct CameraUniform {
  view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

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
  @location(1) @interpolate(perspective) screen_pos: vec2<f32>, // web cannot @interpolate(linear)
};

@vertex 
fn vs_main(
  model: VertexInput,
  instance: InstanceInput,
  @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
  let t = time;
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
  let z_displacement = displacement_strength * 3.0 / (1.0 + exp(1.5 * length(diff) - 4.0));
  // z_displacement is a function that only outputs from 0 to 1, so invert that by subtracting from 1.
  let inverse_z_displacement = 1.0 - z_displacement;


  let displacement = vec4<f32>(xy_displacement, z_displacement, 0.0);

  // Transform the world position with sin/cos and time
  // Do this at an inverse rate to the z_displacement
  let wave_transform = vec4<f32>(
    0.3 * sin(t / 2.0 + initial_world_position.y / 2.0),
    0.3 * cos(t / 2.0 + initial_world_position.x / 2.0),
    0.2 * sin(t + initial_world_position.x + initial_world_position.y),
    0,
  ) * inverse_z_displacement;

  let world_position = initial_world_position + displacement + wave_transform;

  out.clip_position = camera.view_proj * world_position;
  out.screen_pos = vec2<f32>(0.5, 0.5) * (out.clip_position.xy / out.clip_position.w + vec2<f32>(1.0, 1.0));
  return out;
}


// Fragment shader
@group(0) @binding(0)
var t_letter: texture_2d<f32>;
@group(0) @binding(1)
var s_letter: sampler;

@fragment 
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  return textureSample(t_letter, s_letter, in.tex_coords);
}
