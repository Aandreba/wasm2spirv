export fn dot(n: usize, alpha: [*]const f32, beta: [*]const f32, tmp: [*]f32) void {
    const index = gl_GlobalInvocationID(0);
    const size = gl_NumWorkGroups(0);
    var result: f32 = 0;

    var i: usize = index;
    while (i < n) {
        result += alpha[i] * beta[i];
        i += size;
    }

    tmp[index] = result;
}

extern "spir_global" fn gl_GlobalInvocationID(u32) usize;
extern "spir_global" fn gl_NumWorkGroups(u32) usize;
