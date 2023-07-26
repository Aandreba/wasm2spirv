export fn main(og_depth: f32, cond: bool, scale: f32) void {
    var out_depth: f32 = undefined;
    if (cond) {
        out_depth = og_depth + scale;
    } else {
        out_depth = (2 * og_depth) / scale;
    }

    for (0..4) |_| {
        out_depth *= scale;
    }

    gl_FragDepth(out_depth);
}

extern "spir_global" fn gl_FragDepth(f32) void;
