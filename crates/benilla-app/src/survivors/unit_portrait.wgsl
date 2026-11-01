#import bevy_ui::ui_vertex_output::UiVertexOutput

@group(1) @binding(0) var face: texture_2d<f32>;
@group(1) @binding(1) var face_sampler: sampler;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let distance = length(in.uv - vec2<f32>(0.5));
    let edge = max(fwidth(distance), 0.001);
    let alpha = 1.0 - smoothstep(0.5 - edge, 0.5, distance);
    if alpha <= 0.0 { discard; }
    let color = textureSample(face, face_sampler, in.uv);
    // Lift the shadows in the small portrait without washing out bright skin and hair.
    let rgb = clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let lit = pow(rgb, vec3<f32>(0.72));
    return vec4<f32>(lit, color.a * alpha);
}
