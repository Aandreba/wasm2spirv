use rspirv::spirv::ExecutionModel;
use wasm2spirv::{
    config::Config,
    translation::{
        function::{FunctionConfig, ParameterKind, PointerParam},
        module::ModuleBuilder,
    },
};
use wasmparser::WasmFeatures;

#[test]
fn test() {
    let _ = color_eyre::install();
    let config = Config {
        features: WasmFeatures {
            memory64: true,
            ..Default::default()
        },
        functions: vec![FunctionConfig {
            exec_model: Some(ExecutionModel::GLCompute),
            params: vec![
                PointerParam {
                    ty: None,
                    kind: ParameterKind::DescriptorSet { set: 0, binding: 0 },
                },
                PointerParam {
                    ty: None,
                    kind: ParameterKind::DescriptorSet { set: 0, binding: 1 },
                },
            ],
        }],
        ..Default::default()
    };

    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../simple.wat")).unwrap();
    let module = ModuleBuilder::new(config, &wasm).unwrap();
}
