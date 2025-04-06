// VERTEX_FRAGMENT visibility
@group(2) @binding(1)
var<uniform> time: f32;


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
  let model_matrix = mat4x4<f32>(
    instance.model_matrix_0,
    instance.model_matrix_1,
    instance.model_matrix_2,
    instance.model_matrix_3,
  );
  var out: VertexOutput;
  out.tex_coords = model.tex_coords;
  let world_position = model_matrix * vec4<f32>(model.position, 1.0);

  // Note: the w position of screen_position is not 1.0 due to the camera projection. Any offsets need to take that divisor into account
  let screen_position = camera.view_proj * world_position;

  let displacement_strength = displacement_target.w;
  // The screen space displacement
  let screen_displacement = screen_position.w * displacement_target.xy;
  let diff = screen_position.xy - screen_displacement;
  let displacement = displacement_strength * (-1.0 * pow(2.0, -1.0 * length(diff)) + 1.0) * normalize(diff);
  out.clip_position = vec4<f32>(screen_position.xy + displacement.xy * screen_position.w, screen_position.zw);
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
