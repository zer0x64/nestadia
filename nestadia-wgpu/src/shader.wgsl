// Vertex shader
struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] tex_coord: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] tex_coord: vec2<f32>;
};

// Converts the 2D position to a 4D one
[[stage(vertex)]]
fn main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = model.tex_coord;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    return out;
}

// Fragment shader
[[group(0), binding(0)]]
var t_screen: texture_2d<f32>;

[[group(0), binding(1)]]
var s_screen: sampler;

// Apply the texture to the triangles
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return textureSample(t_screen, s_screen, in.tex_coord);
}
