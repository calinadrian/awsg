// FFXGlow, the reference's full-screen glow (`FFXGlow.bls`, `FFXGauss4.bls`):
//   blur = Gauss4(Gauss4(Box4(scene → ¼), horizontal), vertical)
//   out  = lerp(screen, blur, z) + w · blur²
// `w` is the zone's LightParams glow weight, `z` the drunk or underwater haze mix. Tap offsets
// count source texels from texel-centre UVs, with no half-texel shift. The math runs on gamma
// bytes, and every scene read clamps to 1.0 because the float target does not saturate.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var in_tex: texture_2d<f32>;
@group(0) @binding(1) var in_samp: sampler;
// Combine-pass bindings, unbound in the downsample and blur passes.
@group(0) @binding(2) var blur_tex: texture_2d<f32>;
struct FfxCombine {
    // x = zone glow weight `w` (both combines), y = FFXDeath gate (1 while a ghost, no ramp),
    // z = haze mix, w = deband dither on.
    lane: vec4<f32>,
    // xy = the GlowWave phases, z = FSR EASU enabled for the world backdrop, w unused.
    wave: vec4<f32>,
}
@group(0) @binding(3) var<uniform> ffx: FfxCombine;
// The 128×128 wave LUT and its repeat sampler: bound on every combine, sampled only underwater.
@group(0) @binding(4) var wave_tex: texture_2d<f32>;
@group(0) @binding(5) var wave_samp: sampler;



// Box4, full → ¼: four bilinear taps over a 4×4 footprint (`0xce89cc`), each clamped to 1.0 like
// the reference's byte target; an unclamped super-white would square into a hard glow disc.
@fragment
fn fs_downsample(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(in_tex));
    let one = vec4<f32>(1.0);
    return (min(textureSample(in_tex, in_samp, in.uv + vec2<f32>(-1.5, -1.5) * texel), one)
        + min(textureSample(in_tex, in_samp, in.uv + vec2<f32>(0.5, -1.5) * texel), one)
        + min(textureSample(in_tex, in_samp, in.uv + vec2<f32>(0.5, 0.5) * texel), one)
        + min(textureSample(in_tex, in_samp, in.uv + vec2<f32>(-1.5, 0.5) * texel), one))
        * 0.25;
}

// Gauss4: taps at ±0.5 and ±2.5 source texels (0x6cad10), weights from FFXGauss4.bls.
fn gauss4(uv: vec2<f32>, axis: vec2<f32>) -> vec4<f32> {
    let texel = axis / vec2<f32>(textureDimensions(in_tex));
    return textureSample(in_tex, in_samp, uv - 2.5 * texel) * 0.125
        + textureSample(in_tex, in_samp, uv - 0.5 * texel) * 0.375
        + textureSample(in_tex, in_samp, uv + 0.5 * texel) * 0.375
        + textureSample(in_tex, in_samp, uv + 2.5 * texel) * 0.125;
}

@fragment
fn fs_gauss_h(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return gauss4(in.uv, vec2<f32>(1.0, 0.0));
}

@fragment
fn fs_gauss_v(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return gauss4(in.uv, vec2<f32>(0.0, 1.0));
}

fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

// Deviation, opt-in (`WOW_DITHER=1`): a dither against banding, which the reference's 8-bit
// framebuffer lacks. The hash is Bevy's `screen_space_dither`, copied here because Bevy applies
// it only in the tonemapping pass that `Tonemapping::None` skips.
fn screen_space_dither(frag_coord: vec2<f32>) -> vec3<f32> {
    var dither = vec3<f32>(dot(vec2<f32>(171.0, 231.0), frag_coord)).xxx;
    dither = fract(dither.rgb / vec3<f32>(103.0, 71.0, 97.0));
    return (dither - vec3<f32>(0.5)) / 255.0;
}

// The combine's exit: dither in gamma space, where the present-encode rounds, then the frame's
// one decode, which the sRGB present-encode reverses byte-exactly; none under `GAMMA_OUT`.
fn combine_out(outg: vec3<f32>, alpha: f32, frag_coord: vec2<f32>) -> vec4<f32> {
    let dithered = clamp(
        outg + screen_space_dither(frag_coord) * step(0.5, ffx.lane.w),
        vec3<f32>(0.0),
        vec3<f32>(1.0),
    );
#ifdef GAMMA_OUT
    // The UI camera's ground pass stores premultiplied bytes like `ui_quad.wgsl`; the lane decodes.
    return vec4<f32>(dithered * alpha, alpha);
#else
    return vec4<f32>(srgb_to_linear(dithered), alpha);
#endif
}

#ifdef SURVIVORS_UPSCALE
// FSR 1 EASU, adapted to WGSL from AMD's ffx_fsr1.h v1.0.2. Loads use full RGB texels.
// Copyright (c) 2021 Advanced Micro Devices, Inc. All rights reserved.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

fn easu_gradient(north: f32, west: f32, center: f32, east: f32, south: f32) -> vec3<f32> {
    let delta = vec2<f32>(east - west, south - north);
    let middle = vec2<f32>(center);
    let step_size = max(abs(vec2<f32>(east, south) - middle), abs(middle - vec2<f32>(west, north)));
    let consistency = clamp(
        abs(delta) / max(step_size, vec2<f32>(1e-6)),
        vec2<f32>(0.0),
        vec2<f32>(1.0),
    );
    return vec3<f32>(delta, dot(consistency, consistency));
}

fn easu(uv: vec2<f32>) -> vec3<f32> {
    let size = vec2<i32>(textureDimensions(in_tex));
    let pos = uv * vec2<f32>(size) - vec2<f32>(0.5);
    let base = vec2<i32>(floor(pos));
    let part = fract(pos);
    let inverse = vec2<f32>(1.0) - part;
    //   b c
    // e f g h
    // i j k l
    //   n o
    let offsets = array<vec2<i32>, 12>(
        vec2<i32>(0, -1), vec2<i32>(1, -1),
        vec2<i32>(-1, 0), vec2<i32>(0, 0), vec2<i32>(1, 0), vec2<i32>(2, 0),
        vec2<i32>(-1, 1), vec2<i32>(0, 1), vec2<i32>(1, 1), vec2<i32>(2, 1),
        vec2<i32>(0, 2), vec2<i32>(1, 2),
    );
    var colors: array<vec3<f32>, 12>;
    var luma: array<f32, 12>;
    for (var i = 0u; i < 12u; i++) {
        let texel = clamp(base + offsets[i], vec2<i32>(0), size - vec2<i32>(1));
        colors[i] = clamp(textureLoad(in_tex, texel, 0).rgb, vec3<f32>(0.0), vec3<f32>(1.0));
        luma[i] = dot(colors[i], vec3<f32>(0.5, 1.0, 0.5));
    }
    let edge = easu_gradient(luma[0], luma[2], luma[3], luma[4], luma[7]) * inverse.x * inverse.y
        + easu_gradient(luma[1], luma[3], luma[4], luma[5], luma[8]) * part.x * inverse.y
        + easu_gradient(luma[3], luma[6], luma[7], luma[8], luma[10]) * inverse.x * part.y
        + easu_gradient(luma[4], luma[7], luma[8], luma[9], luma[11]) * part.x * part.y;
    let dir_sq = dot(edge.xy, edge.xy);
    let dir = select(
        vec2<f32>(1.0, 0.0),
        edge.xy * inverseSqrt(max(dir_sq, 1.0 / 32768.0)),
        dir_sq >= 1.0 / 32768.0,
    );
    let strength = edge.z * edge.z * 0.25;
    let stretch = 1.0 / max(abs(dir.x), abs(dir.y));
    let kernel = mix(vec2<f32>(1.0), vec2<f32>(stretch, 0.5), strength);
    let lobe = mix(0.5, 0.21, strength);
    var sum = vec3<f32>(0.0);
    var weight_sum = 0.0;
    for (var i = 0u; i < 12u; i++) {
        let offset = vec2<f32>(offsets[i]) - part;
        let tap = vec2<f32>(dot(offset, dir), dot(offset, vec2<f32>(-dir.y, dir.x))) * kernel;
        let distance_sq = min(dot(tap, tap), 1.0 / lobe);
        let base_weight = 0.4 * distance_sq - 1.0;
        let window_weight = lobe * distance_sq - 1.0;
        let weight = (1.5625 * base_weight * base_weight - 0.5625) * window_weight * window_weight;
        sum += colors[i] * weight;
        weight_sum += weight;
    }
    let lo = min(min(colors[3], colors[4]), min(colors[7], colors[8]));
    let hi = max(max(colors[3], colors[4]), max(colors[7], colors[8]));
    return clamp(sum / max(weight_sum, 1e-6), lo, hi);
}
#endif

// Both combines, at the plain or the warped UV; `frag_coord` stays unwarped for the dither.
fn combine_body(uv: vec2<f32>, frag_coord: vec2<f32>) -> vec4<f32> {
    let scene = textureSample(in_tex, in_samp, uv);
    // Clamped like the downsample taps: the screen term must saturate before the glow add.
    var sg = clamp(scene.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
#ifdef SURVIVORS_UPSCALE
    if ffx.wave.z > 0.5 {
        sg = easu(uv);
    }
#endif
    let bg = max(textureSample(blur_tex, in_samp, uv).rgb, vec3<f32>(0.0));
    // Scene alpha passes through; only the transparent-clear create booth reads it.
    if ffx.lane.y > 0.0 {
        // FFXDeath replaces the glow while a ghost, unhazed (one pass slot, `[0xce8bb4]`). The
        // 0.144 blue weight is as shipped; the tint is packed at `0x6cb930`.
        let glowed = sg + ffx.lane.x * bg * bg;
        let luma = clamp(dot(glowed, vec3<f32>(0.299, 0.587, 0.144)), 0.0, 1.0);
        let p = clamp(4.0 * luma * (1.0 - luma), 0.0, 1.0);
        let ghost_tint = vec3<f32>(83.0, 147.0, 168.0) / 255.0; // 0x5393A8
        let outg = min(vec3<f32>(luma) + ghost_tint * p, vec3<f32>(1.0));
        return combine_out(outg, scene.a, frag_coord);
    }
    // The haze fades toward the blur before the glow add; z is the larger of the drunk fraction
    // and 84/255 while submerged.
    let hazed = mix(sg, min(bg, vec3<f32>(1.0)), ffx.lane.z);
    let outg = min(hazed + ffx.lane.x * bg * bg, vec3<f32>(1.0));
    return combine_out(outg, scene.a, frag_coord);
}

@fragment
fn fs_combine(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    return combine_body(in.uv, in.position.xy);
}

// FFXGlowWave, the warp the reference runs instead of FFXGlow while the eye is in any liquid
// (`0x6cc630`, `0x6cb310`): the same combine, both texcoords displaced by a 128×128 sine LUT. The
// reference's per-vertex wave texcoord (`0x7bca80`) is affine in the screen UV, so per fragment
// is exact: uv' = uv · R(10°) · S(W/128, 0.88·H/128) · T(p1, p2), fixed to screen pixels.
const WAVE_ROT_RAD: f32 = 0.174532925; // 10°, the reference's rotation row
const WAVE_LUT_EDGE: f32 = 128.0; // the LUT is 128×128 texels ([0xce89a0])
const WAVE_V_SCALE: f32 = 0.88; // [0xce89c0]: the v axis only; u's scale is 1.0
const WAVE_AMPLITUDE_PX: f32 = 3.0; // full-res pixels, both samples

// The warp's screen-UV offset in UV units; `screen` is the full-res scene size in pixels.
fn wave_offset(uv: vec2<f32>, screen: vec2<f32>) -> vec2<f32> {
    let c = cos(WAVE_ROT_RAD);
    let s = sin(WAVE_ROT_RAD);
    let scale = vec2<f32>(screen.x, WAVE_V_SCALE * screen.y) / WAVE_LUT_EDGE;
    let tex = vec2<f32>(c * uv.x - s * uv.y, s * uv.x + c * uv.y) * scale + ffx.wave.xy;
    // Sampled, not evaluated: the reference's sine is a filtered LUT. The sampler must repeat,
    // as the texcoord spans about 10 cycles; the unbias matches the shipped ps_2_0 shader.
    let d = (textureSample(wave_tex, wave_samp, tex).rg - vec2<f32>(0.5)) * 2.0;
    return d * WAVE_AMPLITUDE_PX / screen;
}

@fragment
fn fs_combine_wave(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let screen = vec2<f32>(textureDimensions(in_tex));
    return combine_body(in.uv + wave_offset(in.uv, screen), in.position.xy);
}
