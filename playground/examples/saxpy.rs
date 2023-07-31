#[no_mangle]
pub unsafe extern "C" fn saxpy(n: usize, alpha: f32, x: *const f32, y: *mut f32) {
    let x = unsafe { core::slice::from_raw_parts(x, n) };
    let y = unsafe { core::slice::from_raw_parts_mut(y, n) };

    let mut i = gl_GlobalInvocationID(0);
    let size = gl_NumWorkGroups(0);

    while i < n {
        y[i] += alpha * x[i];
        i += size;
    }
}

#[link(wasm_import_module = "spir_global")]
extern "C" {
    fn gl_GlobalInvocationID(n: u32) -> usize;
    fn gl_NumWorkGroups(n: u32) -> usize;
}
