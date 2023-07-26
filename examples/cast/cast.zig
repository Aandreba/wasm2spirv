const std = @import("std");
const math = std.math;

export fn main(n: usize, alpha: f32, x: [*]const f32, y: [*]f64) void {
    y[n] = switch (total_cmp_f32(alpha, x[n])) {
        .lt, .eq => @floatCast(x[n]),
        else => @floatCast(alpha),
    };
}

/// Based on Rust's [`total_cmp`](https://doc.rust-lang.org/stable/src/core/num/f32.rs.html#1374)
inline fn total_cmp_f32(x: f32, y: f32) math.Order {
    var left: i32 = @bitCast(x);
    var right: i32 = @bitCast(y);

    left ^= @as(i32, @intCast(@as(u32, @intCast(left >> 31)) >> 1));
    right ^= @as(i32, @intCast(@as(u32, @intCast(right >> 31)) >> 1));

    return math.order(left, right);
}
