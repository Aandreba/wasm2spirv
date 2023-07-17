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
            1,
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
            2,
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
    //let wat = include_str!("saxpy.wat");
    let wasm = wat::parse_str(include_str!("../simple.wat")).unwrap();
    let module = ModuleBuilder::new(config, &wasm).unwrap();
    let spirv = module.translate().unwrap();
    println!("{}", spirv.module().disassemble())
}
