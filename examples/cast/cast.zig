const std = @import("std");
const math = std.math;

export fn main(n: usize, alpha: f32, x: [*]const f32, y: [*]f64) void {
    var i = gl_GlobalInvocationID(0);
    const size = gl_NumWorkGroups(0);

    while (i < n) {
        y[i] = switch (total_cmp_f32(alpha, x[i])) {
            .lt, .eq => @as(f64, @floatCast(x[i])),
            else => @as(f64, @floatCast(alpha)),
        };

        i += size;
    }
}

/// Based on Rust's [`total_cmp`](https://doc.rust-lang.org/stable/src/core/num/f32.rs.html#1374)
fn total_cmp_f32(x: f32, y: f32) math.Order {
    var left: i32 = @bitCast(x);
    var right: i32 = @bitCast(y);

    left ^= @as(i32, @intCast(@as(u32, @intCast(left >> 31)) >> 1));
    right ^= @as(i32, @intCast(@as(u32, @intCast(right >> 31)) >> 1));

    return math.order(left, right);
}

extern "spir_global" fn gl_GlobalInvocationID(u32) usize;
extern "spir_global" fn gl_NumWorkGroups(u32) usize;
