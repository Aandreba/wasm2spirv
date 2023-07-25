use super::{
    module::{GlobalVariable, ModuleBuilder},
    Operation,
};
use crate::{
    decorator::VariableDecorator,
    error::{Error, Result},
    fg::{module::CallableFunction, values::pointer::Pointer},
    r#type::{CompositeType, PointerSize, ScalarType, Type},
};
use rspirv::spirv::{BuiltIn, StorageClass};
use std::rc::Rc;
use wasmparser::TypeRef;

pub enum ImportResult {
    Global(GlobalVariable),
    Func(CallableFunction),
}

pub fn translate_spir_global<'a>(
    name: &'a str,
    ty: TypeRef,
    module: &mut ModuleBuilder,
) -> Result<Option<ImportResult>> {
    // TODO gl_LocalInvocationIndex
    let result = match name {
        // Fragment shaders
        "gl_FragDepth" => import_output(BuiltIn::FragDepth, ScalarType::F32, ty, module),

        // Compute Shaders
        "gl_NumWorkGroups" => import_uint3_input(BuiltIn::NumWorkgroups, ty, module),
        "gl_WorkGroupID" => import_uint3_input(BuiltIn::WorkgroupId, ty, module),
        "gl_WorkGroupSize" => import_uint3_input(BuiltIn::WorkgroupSize, ty, module),
        "gl_LocalInvocationID" => import_uint3_input(BuiltIn::LocalInvocationId, ty, module),
        "gl_GlobalInvocationID" => import_uint3_input(BuiltIn::GlobalInvocationId, ty, module),
        _ => return Ok(None),
    };

    return result.map(Some);
}

fn import_output(
    builtin: BuiltIn,
    output_type: impl Into<Type>,
    ty: TypeRef,
    module: &mut ModuleBuilder,
) -> Result<ImportResult> {
    let output_type = output_type.into();

    let var = Rc::new(Pointer::new_variable(
        PointerSize::Skinny,
        StorageClass::Output,
        output_type.clone(),
        None,
        [VariableDecorator::BuiltIn(builtin)],
    ));

    return Ok(match ty {
        TypeRef::Func(_) => {
            module.hidden_global_variables.push(var.clone());
            ImportResult::Func(CallableFunction::callback(
                move |block, function, module| {
                    if let Some(ref mut entry_point) = function.entry_point {
                        if !entry_point.interface.iter().any(|x| Rc::ptr_eq(x, &var)) {
                            entry_point.interface.push(var.clone());
                        }
                    }

                    let value = block.stack_pop(output_type.clone(), module)?;
                    function.anchors.push(Operation::Store {
                        target: var.clone(),
                        value,
                        log2_alignment: None,
                    });

                    Ok(())
                },
            ))
        }
        _ => return Err(Error::unexpected()),
    });
}

fn import_uint3_input(
    builtin: BuiltIn,
    ty: TypeRef,
    module: &mut ModuleBuilder,
) -> Result<ImportResult> {
    let var = Rc::new(Pointer::new_variable(
        PointerSize::Skinny,
        StorageClass::Input,
        CompositeType::vector(ScalarType::I32, 3),
        None,
        [VariableDecorator::BuiltIn(builtin)],
    ));

    return Ok(match ty {
        TypeRef::Func(_) => {
            module.hidden_global_variables.push(var.clone());
            ImportResult::Func(CallableFunction::callback(
                move |block, function, module| {
                    if let Some(ref mut entry_point) = function.entry_point {
                        if !entry_point.interface.iter().any(|x| Rc::ptr_eq(x, &var)) {
                            entry_point.interface.push(var.clone());
                        }
                    }

                    let index = block.stack_pop(ScalarType::I32, module)?.into_integer()?;
                    let vector = var.clone().load(None, block, module)?.into_vector()?;
                    block.stack_push(vector.extract(index));
                    Ok(())
                },
            ))
        }
        _ => return Err(Error::unexpected()),
    });
}
