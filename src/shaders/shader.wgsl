// Vertex shader
struct CameraUniform {
  view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
  @location(0) position: vec3<f32>,
  @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
  @builtin(position) clip_position: vec4<f32>,
  @location(0) tex_coords: vec2<f32>,
};

@vertex 
fn vs_main(
  model: VertexInput,
  @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
  var out: VertexOutput;
  out.tex_coords = model.tex_coords;
  out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
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
