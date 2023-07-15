use super::{module::ModuleBuilder, values::pointer::Pointer};
use crate::{
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::Type,
};
use rspirv::spirv::{ExecutionModel, StorageClass};
use std::rc::Rc;
use wasmparser::{FuncType, FunctionBody, ValType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FunctionBuilder {
    pub local_variables: Box<[Rc<Pointer>]>,
    pub return_type: Option<Type>,
}

impl FunctionBuilder {
    pub fn new(
        config: &FunctionConfig,
        ty: &FuncType,
        body: FunctionBody,
        module: &mut ModuleBuilder,
    ) -> Result<Self> {
        if ty.results().len() >= 2 {
            return Err(Error::msg("Function can only have a single result value"));
        }

        let mut locals = Vec::new();
        let return_type = ty.results().get(0).cloned().map(Type::from);

        // Add function params as local variables
        for (wasm_ty, param) in ty.params().iter().zip(config.params.iter()) {
            let ty = param.ty.clone().unwrap_or_else(|| Type::from(*wasm_ty));
            let decorators = match param.kind {
                ParameterKind::FunctionParameter => todo!(),
                ParameterKind::DescriptorSet { set, binding } => vec![
                    VariableDecorator::DesctiptorSet(set),
                    VariableDecorator::Binding(binding),
                ],
            };

            let variable = Rc::new(Pointer::new_variable(
                StorageClass::Function,
                ty,
                decorators,
            ));

            locals.push(variable);
        }

        // Create local variables
        let mut locals_reader = body.get_locals_reader()?;
        for _ in 0..locals_reader.get_count() {
            let (count, ty) = locals_reader.read()?;
            let ty = match ty {
                ValType::I32 if !module.wasm_memory64 => Type::Schrodinger,
                ValType::I64 if module.wasm_memory64 => Type::Schrodinger,
                _ => Type::from(ty),
            };

            locals.reserve(count as usize);
            for _ in 0..count {
                locals.push(Rc::new(Pointer::new_variable(
                    StorageClass::Function,
                    ty.clone(),
                    None,
                )));
            }
        }

        let result = Self {
            local_variables: locals.into_boxed_slice(),
            return_type,
        };

        return Ok(result);
    }
}

#[derive(Debug, Clone)]
pub struct FunctionConfig {
    pub exec_model: ExecutionModel,
    pub params: Vec<PointerParam>,
}

#[derive(Debug, Clone)]
pub struct PointerParam {
    pub ty: Option<Type>,
    pub kind: ParameterKind,
}

#[derive(Debug, Clone)]
pub enum ParameterKind {
    FunctionParameter,
    DescriptorSet { set: u32, binding: u32 },
}
