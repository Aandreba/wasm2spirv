export fn main(og_color: f32, cond: bool, scale: f32) void {
    var out_color: f32 = undefined;
    if (cond) {
        out_color = og_color + scale;
    } else {
        out_color = (2 * og_color) / scale;
    }

    for (0..4) |_| {
        out_color *= scale;
    }

    gl_FragDepth(out_color);
}

extern "spir_global" fn gl_FragDepth(f32) void;
