export fn main(og_color: f32, cond: bool, scale: f32, color: *f32) void {
    var out_color: f32 = undefined;
    if (cond) {
        out_color = og_color + scale;
    } else {
        out_color = (2 * og_color) / scale;
    }

    for (0..4) |_| {
        out_color *= scale;
    }

    color.* = out_color;
}
