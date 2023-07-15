use wasm2spirv::{config::Config, translation::module::ModuleBuilder};
use wasmparser::WasmFeatures;

#[test]
fn test() {
    let _ = color_eyre::install();
    let config = Config {
        features: WasmFeatures {
            memory64: true,
            ..Default::default()
        },
        ..Default::default()
    };

    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../saxpy.wat")).unwrap();
    let module = ModuleBuilder::new(config, &wasm).unwrap();
}
