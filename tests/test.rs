use rspirv::binary::{Assemble, Disassemble};
use std::mem::ManuallyDrop;
use wasm2spirv::fg::module::ModuleBuilder;

#[test]
fn test() -> color_eyre::Result<()> {
    let _ = color_eyre::install();

    let config = serde_json::from_str(include_str!("../examples/cast/cast.json"))?;

    let wasm = wat::parse_bytes(include_bytes!("../examples/cast/cast.wat"))?;
    let module = ModuleBuilder::new(config, &wasm)?;
    let spirv = module.translate().unwrap();

    let module = spirv.module();
    println!("{}", module.disassemble());

    let mut content = ManuallyDrop::new(module.assemble());
    let content = unsafe {
        Vec::from_raw_parts(
            content.as_mut_ptr().cast::<u8>(),
            4 * content.len(),
            4 * content.capacity(),
        )
    };

    std::fs::write("examples/out/test.spv", content)?;
    return Ok(());
}
