use std::collections::BTreeMap;

use rspirv::{binary::Disassemble, spirv::ExecutionModel};
use wasm2spirv::{
    ast::{
        function::{FunctionConfig, ParameterKind, PointerParam},
        module::ModuleBuilder,
    },
    config::{AddressingModel, CapabilityModel, Config},
};
use wasmparser::WasmFeatures;

#[test]
fn test() {
    let _ = color_eyre::install();
    let mut config = Config::builder();
    config.function(0).params = BTreeMap::from([
        (
            0,
            PointerParam {
                kind: ParameterKind::DescriptorSet { set: 0, binding: 0 },
                ..Default::default()
            },
        ),
        (
            1,
            PointerParam {
                kind: ParameterKind::DescriptorSet { set: 0, binding: 1 },
                ..Default::default()
            },
        ),
    ]);

    let config = config.build();

    /*
    let config = Config::new(
        WasmFeatures {
            memory64: true,
            ..WasmFeatures::default()
        },
        AddressingModel::Logical,
        CapabilityModel::default(),
        [(
            2,
            FunctionConfig {
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
            },
        )],
    );
    */

    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../simple.wat")).unwrap();
    let module = ModuleBuilder::new(config, &wasm).unwrap();
    let spirv = module.translate().unwrap();
    println!("{}", spirv.module().disassemble())
}
