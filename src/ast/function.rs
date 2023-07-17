use super::{
    block::{BlockBuilder, BlockReader},
    module::ModuleBuilder,
    values::pointer::Pointer,
    Operation,
};
use crate::{
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::Type,
};
use once_cell::unsync::OnceCell;
use rspirv::spirv::{ExecutionModel, StorageClass};
use std::{borrow::Cow, cell::Cell, collections::BTreeMap, rc::Rc};
use wasmparser::{FuncType, FunctionBody, ValType};

#[derive(Debug, Clone, PartialEq)]
pub enum SchrodingerKind {
    Integer,
    Pointer(StorageClass, Type),
}

/// May be a pointer or an integer, but you won't know until you try to store into it.
#[derive(Debug, Clone, PartialEq)]
pub struct Schrodinger {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub kind: OnceCell<SchrodingerKind>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Storeable {
    Pointer(Rc<Pointer>),
    Schrodinger(Rc<Schrodinger>),
}

#[derive(Debug, Default)]
pub struct FunctionBuilder {
    pub local_variables: Box<[Storeable]>,
    pub return_type: Option<Type>,
    /// Instructions who's order **must** be followed
    pub anchors: Vec<Operation>,
}

impl FunctionBuilder {
    pub fn new<'a>(
        config: &FunctionConfig,
        ty: &FuncType,
        body: FunctionBody<'a>,
        module: &mut ModuleBuilder,
    ) -> Result<(Self, BlockBuilder<'a>)> {
        if ty.results().len() >= 2 {
            return Err(Error::msg("Function can only have a single result value"));
        }

        let mut locals = Vec::new();
        let return_type = ty.results().get(0).cloned().map(Type::from);

        // Add function params as local variables
        for (wasm_ty, i) in ty.params().iter().zip(0..) {
            let param = config
                .params
                .get(&i)
                .map_or_else(Cow::default, Cow::Borrowed);

            let ty = param.ty.clone().unwrap_or_else(|| Type::from(*wasm_ty));
            let decorators = match param.kind {
                ParameterKind::FunctionParameter => Vec::new(),
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

            locals.push(Storeable::Pointer(variable));
        }

        // Create local variables
        let mut locals_reader = body.get_locals_reader()?;
        for _ in 0..locals_reader.get_count() {
            let (count, ty) = locals_reader.read()?;
            locals.reserve(count as usize);

            if matches!(ty, ValType::I32 if !module.wasm_memory64)
                || matches!(ty, ValType::I64 if module.wasm_memory64)
            {
                for _ in 0..count {
                    locals.push(Storeable::Schrodinger(Rc::new(Schrodinger {
                        translation: Cell::new(None),
                        kind: OnceCell::new(),
                    })));
                }
            } else {
                let ty = Type::from(ty);
                for _ in 0..count {
                    locals.push(Storeable::Pointer(Rc::new(Pointer::new_variable(
                        StorageClass::Function,
                        ty.clone(),
                        None,
                    ))));
                }
            }
        }

        let mut result = Self {
            local_variables: locals.into_boxed_slice(),
            return_type,
            anchors: Vec::new(),
        };

        let reader = BlockReader::new(body.get_operators_reader()?);
        let block = BlockBuilder::new(reader, result.return_type.clone(), &mut result, module)?;

        return Ok((result, block));
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionConfig {
    pub exec_model: Option<ExecutionModel>,
    pub params: BTreeMap<u32, PointerParam>,
}

#[derive(Debug, Clone, Default)]
pub struct PointerParam {
    pub ty: Option<Type>,
    pub kind: ParameterKind,
}

#[derive(Debug, Clone, Default)]
pub enum ParameterKind {
    #[default]
    FunctionParameter,
    DescriptorSet {
        set: u32,
        binding: u32,
    },
}
