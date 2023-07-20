use super::module::{GlobalVariable, ModuleBuilder};
use crate::{
    decorator::VariableDecorator,
    error::{Error, Result},
    fg::{module::CallableFunction, values::pointer::Pointer},
    r#type::{CompositeType, ScalarType},
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
        "gl_NumWorkGroups" => import_uint3(BuiltIn::NumWorkgroups, ty, module),
        "gl_WorkGroupID" => import_uint3(BuiltIn::WorkgroupId, ty, module),
        "gl_WorkGroupSize" => import_uint3(BuiltIn::WorkgroupSize, ty, module),
        "gl_LocalInvocationID" => import_uint3(BuiltIn::LocalInvocationId, ty, module),
        "gl_GlobalInvocationID" => import_uint3(BuiltIn::GlobalInvocationId, ty, module),
        _ => return Ok(None),
    };

    return result.map(Some);
}

fn import_uint3(builtin: BuiltIn, ty: TypeRef, module: &mut ModuleBuilder) -> Result<ImportResult> {
    let var = Rc::new(Pointer::new_variable(
        StorageClass::Input,
        CompositeType::vector(ScalarType::I32, 3),
        Some(VariableDecorator::BuiltIn(builtin)),
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
