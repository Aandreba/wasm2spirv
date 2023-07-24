export fn main(n: usize, alpha: f32, x: [*]const f32, y: [*]f32) void {
    var i = gl_GlobalInvocationID(0);
    const size = gl_NumWorkGroups(0);

    while (i < n) {
        y[i] += @min(alpha, x[i]);
        i += size;
    }
}

extern "spir_global" fn gl_GlobalInvocationID(u32) usize;
extern "spir_global" fn gl_NumWorkGroups(u32) usize;
