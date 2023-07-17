use super::{
    block::{translate_function, BlockReader},
    module::ModuleBuilder,
    values::{integer::Integer, pointer::Pointer, Value},
    Operation,
};
use crate::{
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::Type,
};
use once_cell::unsync::OnceCell;
use rspirv::spirv::{Capability, ExecutionModel, StorageClass};
use std::{borrow::Cow, cell::Cell, collections::BTreeMap, rc::Rc};
use wasmparser::{FuncType, FunctionBody, ValType};

/// May be a pointer or an integer, but you won't know until you try to store into it.
#[derive(Debug, Clone, PartialEq)]
pub struct Schrodinger {
    pub variable: OnceCell<Rc<Pointer>>,
}

impl Schrodinger {
    pub fn store_integer(
        &self,
        value: Rc<Integer>,
        module: &mut ModuleBuilder,
    ) -> Result<Operation> {
        let variable = self.variable.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                StorageClass::Function,
                module.isize_type(),
                None,
            ))
        });

        match &variable.pointee {
            Type::Scalar(x) if x == &module.isize_type() => {
                variable.clone().store(value, None, module)
            }
            Type::Pointer(storage_class, pointee) => {
                let value = value.to_pointer(*storage_class, Type::clone(pointee), module)?;
                variable.clone().store(value, None, module)
            }
            _ => return Err(Error::unexpected()),
        }
    }

    pub fn store_pointer(
        &self,
        value: Rc<Pointer>,
        module: &mut ModuleBuilder,
    ) -> Result<Operation> {
        let variable = self.variable.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                StorageClass::Function,
                Type::pointer(value.storage_class, value.pointee.clone()),
                None,
            ))
        });

        match &variable.pointee {
            Type::Scalar(x) if x == &module.isize_type() => {
                let value = value.to_integer(module)?;
                variable.clone().store(value, None, module)
            }
            Type::Pointer(sch_storage_class, pointee)
                if sch_storage_class == &value.storage_class =>
            {
                let value = value.cast(Type::clone(pointee));
                variable.clone().store(value, None, module)
            }
            _ => return Err(Error::unexpected()),
        }
    }

    pub fn load(&self, module: &mut ModuleBuilder) -> Result<Value> {
        match self.variable.get() {
            Some(ptr) => ptr.clone().load(None, module),
            None => return Err(Error::msg("Schrodinger variable is still uninitialized")),
        }
    }

    pub fn load_pointer(
        &self,
        storage_class: StorageClass,
        pointee: Type,
        module: &mut ModuleBuilder,
    ) -> Result<Rc<Pointer>> {
        match self.variable.get() {
            Some(ptr) => match ptr.pointee {
                Type::Pointer(sch_storage_class, _) if sch_storage_class == storage_class => {
                    match ptr.clone().load(None, module)? {
                        Value::Pointer(ptr) => Ok(ptr.cast(pointee)),
                        _ => return Err(Error::unexpected()),
                    }
                }

                Type::Scalar(x) if x == module.isize_type() => {
                    match ptr.clone().load(None, module)? {
                        Value::Integer(int) => {
                            int.to_pointer(storage_class, pointee, module).map(Rc::new)
                        }
                        _ => return Err(Error::unexpected()),
                    }
                }

                _ => return Err(Error::unexpected()),
            },
            None => return Err(Error::msg("Schrodinger variable is still uninitialized")),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Storeable {
    Pointer {
        is_extern_pointer: bool,
        pointer: Rc<Pointer>,
    },
    Schrodinger(Rc<Schrodinger>),
}

#[derive(Debug, Default)]
pub struct FunctionBuilder {
    pub(crate) function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
    pub parameters: Box<[Type]>,
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
    ) -> Result<Self> {
        if ty.results().len() >= 2 {
            return Err(Error::msg("Function can only have a single result value"));
        }

        let mut params = Vec::new();
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
                ParameterKind::FunctionParameter => {
                    params.push(ty.clone());
                    Vec::new()
                }
                ParameterKind::DescriptorSet { set, binding } => vec![
                    VariableDecorator::DesctiptorSet(set),
                    VariableDecorator::Binding(binding),
                ],
            };

            let variable = Rc::new(Pointer::new_variable(param.storage_class, ty, decorators));

            locals.push(Storeable::Pointer {
                pointer: variable,
                is_extern_pointer: matches!(param, Cow::Borrowed(_)),
            });
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
                    let storeable = Storeable::Schrodinger(Rc::new(Schrodinger {
                        variable: OnceCell::new(),
                    }));

                    locals.push(storeable);
                }
            } else {
                let ty = Type::from(ty);
                for _ in 0..count {
                    let pointer = Rc::new(Pointer::new_variable(
                        StorageClass::Function,
                        ty.clone(),
                        None,
                    ));

                    locals.push(Storeable::Pointer {
                        pointer,
                        is_extern_pointer: false,
                    });
                }
            }
        }

        let mut result = Self {
            function_id: Rc::new(Cell::new(None)),
            anchors: Vec::new(),
            parameters: params.into_boxed_slice(),
            local_variables: locals.into_boxed_slice(),
            return_type,
        };

        let reader = BlockReader::new(body.get_operators_reader()?);
        translate_function(reader, result.return_type.clone(), &mut result, module)?;

        return Ok(result);
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionConfig {
    pub exec_model: Option<ExecutionModel>,
    pub params: BTreeMap<u32, PointerParam>,
}

#[derive(Debug, Clone)]
pub struct PointerParam {
    pub ty: Option<Type>,
    pub storage_class: StorageClass,
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

impl Default for PointerParam {
    fn default() -> Self {
        Self {
            ty: Default::default(),
            storage_class: StorageClass::Function,
            kind: Default::default(),
        }
    }
}
