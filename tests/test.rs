use rspirv::{binary::Disassemble, spirv::StorageClass};
use std::collections::BTreeMap;
use wasm2spirv::{
    ast::{
        function::{ParameterKind, PointerParam},
        module::ModuleBuilder,
    },
    config::Config,
    r#type::{CompositeType, ScalarType, Type},
};

#[test]
fn test() {
    let _ = color_eyre::install();
    let mut config = Config::builder();
    config.function(0).params = BTreeMap::from([
        (
            0,
            PointerParam {
                ty: Some(Type::Composite(CompositeType::StructuredArray(Box::new(
                    ScalarType::I32,
                )))),
                storage_class: StorageClass::StorageBuffer,
                kind: ParameterKind::DescriptorSet { set: 0, binding: 0 },
                ..Default::default()
            },
        ),
        (
            1,
            PointerParam {
                ty: Some(Type::Composite(CompositeType::StructuredArray(Box::new(
                    ScalarType::I32,
                )))),
                storage_class: StorageClass::StorageBuffer,
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
