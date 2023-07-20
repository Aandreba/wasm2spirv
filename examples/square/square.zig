export fn Main(input: [*]const i32, output: [*]i32) void {
    const idx = gl_GlobalInvocationID(0);
    output[idx] = input[idx] * input[idx];
}

extern "spir_global" fn gl_GlobalInvocationID(u32) usize;
