export fn main(og_color: f32, cond: bool, scale: f32, color: *f32) void {
    if (cond) {
        color.* = og_color + scale;
    } else {
        color.* = (2 * og_color) / scale;
    }

    for (0..4) |_| {
        color.* *= scale;
    }
}
