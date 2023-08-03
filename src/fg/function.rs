use super::{
    block::{translate_block, BlockBuilder, BlockReader, StackValue},
    module::ModuleBuilder,
    values::{integer::Integer, pointer::Pointer, Value},
    End, Label, Operation,
};
use crate::{
    config::ConfigBuilder,
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::{PointerSize, ScalarType, Type},
    version::Version,
};
use elor::Either;
use once_cell::unsync::OnceCell;
use rspirv::spirv::{Capability, ExecutionModel, StorageClass};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::Cell, collections::VecDeque, rc::Rc};
use vector_mapp::vec::VecMap;
use wasmparser::{Export, FuncType, FunctionBody, ValType};

/// May be a pointer or an integer, but you won't know until you try to store into it.
#[derive(Debug, Clone)]
pub struct Schrodinger {
    pub pointer: OnceCell<Rc<Pointer>>,
    pub offset: OnceCell<Rc<Pointer>>,
    pub integer: OnceCell<Rc<Pointer>>,
}

impl Schrodinger {
    fn offset_variable(&self, module: &ModuleBuilder) -> &Rc<Pointer> {
        self.offset.get_or_init(|| {
            let init = Rc::new(Integer::new_constant_usize(0, module));
            Rc::new(Pointer::new_variable(
                PointerSize::Skinny,
                StorageClass::Function,
                module.isize_type(),
                Some(init.into()),
                [],
            ))
        })
    }

    fn integer_variable(&self, module: &ModuleBuilder) -> &Rc<Pointer> {
        self.integer.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                PointerSize::Skinny,
                StorageClass::Function,
                module.isize_type(),
                None,
                [],
            ))
        })
    }

    pub fn store_integer(
        &self,
        value: Rc<Integer>,
        block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<Operation> {
        self.integer_variable(module)
            .clone()
            .store(value, None, block, module)
    }

    pub fn store_pointer(
        &self,
        value: Rc<Pointer>,
        block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<(Operation, Option<Operation>)> {
        let (value, offset) = value.take_byte_offset();
        let pointer = self.pointer.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                PointerSize::Skinny,
                StorageClass::Function,
                Type::pointer(
                    value.kind.to_pointer_size(),
                    value.storage_class,
                    value.pointee.clone(),
                ),
                None,
                [],
            ))
        });

        let offset = if let Some(offset) = offset {
            Some(
                self.offset_variable(module)
                    .clone()
                    .store(offset, None, block, module)?,
            )
        } else if let Some(sch_offset) = self.offset.get() {
            let zero = Rc::new(Integer::new_constant_usize(0, module));
            Some(sch_offset.clone().store(zero, None, block, module)?)
        } else {
            None
        };

        let value = pointer.clone().store(value, None, block, module)?;
        Ok((value, offset))
    }

    pub fn load(
        &self,
        block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<Option<StackValue>> {
        let mut pointer_variable = match self.pointer.get() {
            Some(pointer) => Some(pointer.clone().load(None, block, module)?),
            None => None,
        };

        if let (Some(pointer_variable), Some(offset)) =
            (pointer_variable.as_mut(), self.offset.get())
        {
            let offset = offset.clone().load(None, block, module)?.into_integer()?;
            *pointer_variable = pointer_variable.clone().i_add(offset, module)?;
        }

        let mut loaded_integer = None;
        if let Some(integer) = self.integer.get() {
            loaded_integer = Some(integer.clone().load(None, block, module)?.into_integer()?);
        }

        return Ok(Some(match (pointer_variable, loaded_integer) {
            (Some(pointer_variable), Some(loaded_integer)) => StackValue::Schrodinger {
                pointer_variable: pointer_variable.into_pointer()?,
                loaded_integer,
            },
            (Some(pointer_variable), None) => StackValue::Value(pointer_variable),
            (None, Some(loaded_integer)) => StackValue::Value(Value::Integer(loaded_integer)),
            (None, None) => return Ok(None),
        }));
    }
}

#[derive(Debug, Clone)]
pub enum Storeable {
    Pointer {
        variable: Rc<Pointer>,
        integer_variable: Option<Rc<Pointer>>, // integer.is_some() --> is_extern_pointer
    },
    Schrodinger(Rc<Schrodinger>),
}

#[derive(Debug, Clone)]
pub struct EntryPoint<'a> {
    pub execution_model: ExecutionModel,
    pub execution_modes: Box<[ExecutionMode]>,
    pub name: &'a str,
    pub interface: Vec<Rc<Pointer>>,
}

#[derive(Debug, Default)]
pub struct FunctionBuilder<'a> {
    pub(crate) function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
    pub entry_point: Option<EntryPoint<'a>>,
    pub parameters: Box<[Value]>,
    pub local_variables: Box<[Storeable]>,
    pub return_type: Option<Type>,
    /// Instructions who's order **must** be followed
    pub anchors: Vec<Operation>,
    pub variable_initializers: Box<[Operation]>,
    pub outside_vars: Box<[Rc<Pointer>]>,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
        export: Option<Export<'a>>,
        config: &FunctionConfig,
        ty: &FuncType,
        body: FunctionBody<'a>,
        module: &mut ModuleBuilder,
    ) -> Result<Self> {
        if ty.results().len() >= 2 {
            return Err(Error::msg("Function can only have a single result value"));
        }

        let mut interface = Vec::new();
        let mut params = Vec::new();
        let mut locals = Vec::new();
        let mut outside_vars = Vec::new();
        let mut variable_initializers = Vec::new();
        let return_type = ty.results().get(0).cloned().map(Type::from);

        // Add function params as local variables
        for (wasm_ty, i) in ty.params().iter().zip(0..) {
            let param = config
                .params
                .get(&i)
                .map_or_else(Cow::default, Cow::Borrowed);

            let (ty, pointer_size, storage_class, integer_variable) =
                match param.ty.clone().unwrap_or_else(|| Type::from(*wasm_ty)) {
                    Type::Pointer {
                        size,
                        storage_class,
                        pointee,
                    } => (
                        *pointee,
                        size,
                        storage_class,
                        Some(Rc::new(Pointer::new_variable(
                            PointerSize::Skinny,
                            StorageClass::Function,
                            ScalarType::Isize(module),
                            None,
                            [],
                        ))),
                    ),
                    ty => (ty, PointerSize::Skinny, param.kind.storage_class(), None),
                };

            let variable = match param.kind {
                ParameterKind::FunctionParameter => {
                    let param = Value::function_parameter(ty.clone());
                    let var = Rc::new(Pointer::new_variable(
                        pointer_size,
                        StorageClass::Function,
                        ty,
                        None,
                        Vec::new(),
                    ));

                    variable_initializers.push(Operation::Store {
                        target: var.clone(),
                        value: param.clone(),
                        log2_alignment: None,
                    });
                    params.push(param);
                    var
                }

                ParameterKind::Input(location) => {
                    let mut decorators = vec![VariableDecorator::Location(location)];
                    match ty {
                        Type::Scalar(_) => decorators.push(VariableDecorator::Flat),
                        _ => {}
                    };

                    let param = Rc::new(Pointer::new_variable(
                        pointer_size,
                        storage_class,
                        ty.clone(),
                        None,
                        decorators,
                    ));
                    outside_vars.push(param.clone());
                    interface.push(param.clone());

                    let variable = Rc::new(Pointer::new_variable(
                        pointer_size,
                        StorageClass::Function,
                        ty,
                        None,
                        Vec::new(),
                    ));

                    variable_initializers.push(Operation::Copy {
                        src: param,
                        src_log2_alignment: None,
                        dst: variable.clone(),
                        dst_log2_alignment: None,
                    });

                    variable
                }

                ParameterKind::Output(location) => {
                    let decorators = vec![VariableDecorator::Location(location)];
                    let param = Rc::new(Pointer::new_variable(
                        pointer_size,
                        storage_class,
                        ty,
                        None,
                        decorators,
                    ));
                    param
                }

                ParameterKind::DescriptorSet { set, binding, .. } => {
                    let param = Rc::new(Pointer::new_variable(
                        pointer_size,
                        storage_class,
                        ty,
                        None,
                        vec![
                            VariableDecorator::DesctiptorSet(set),
                            VariableDecorator::Binding(binding),
                        ],
                    ));
                    param
                }
            };

            if variable.storage_class != StorageClass::Function {
                outside_vars.push(variable.clone());
                if module.version >= Version::V1_4
                    || matches!(storage_class, StorageClass::Input | StorageClass::Output)
                {
                    interface.push(variable.clone())
                }
            }

            locals.push(Storeable::Pointer {
                variable,
                integer_variable,
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
                        pointer: OnceCell::new(),
                        offset: OnceCell::new(),
                        integer: OnceCell::new(),
                    }));
                    locals.push(storeable);
                }
            } else {
                let ty = Type::from(ty);
                for _ in 0..count {
                    let pointer = Rc::new(Pointer::new_variable(
                        PointerSize::Skinny,
                        StorageClass::Function,
                        ty.clone(),
                        None,
                        [],
                    ));

                    locals.push(Storeable::Pointer {
                        variable: pointer,
                        integer_variable: None,
                    });
                }
            }
        }

        let entry_point = match (export, config.execution_model) {
            (Some(export), Some(execution_model)) => Some(EntryPoint {
                execution_model,
                execution_modes: config.execution_modes.clone().into_boxed_slice(),
                name: export.name,
                interface, // TODO
            }),
            (None, Some(_)) => todo!(),
            _ => None,
        };

        let mut result = Self {
            anchors: Vec::new(),
            parameters: params.into_boxed_slice(),
            local_variables: locals.into_boxed_slice(),
            outside_vars: outside_vars.into_boxed_slice(),
            variable_initializers: variable_initializers.into_boxed_slice(),
            function_id,
            entry_point,
            return_type,
        };

        let reader = BlockReader::new(body.get_operators_reader()?);
        translate_block(
            reader,
            VecDeque::new(),
            End::Return(result.return_type.clone()),
            &mut result,
            module,
        )?;

        return Ok(result);
    }

    pub fn block_of(&self, op: &Operation) -> Option<&Rc<Label>> {
        let mut current_blocks = Vec::new();

        for anchor in self.anchors.iter() {
            if anchor.ptr_eq(op) {
                return if current_blocks.is_empty() {
                    None
                } else {
                    Some(current_blocks.remove(current_blocks.len() - 1))
                };
            } else if let Operation::Label(label) = anchor {
                current_blocks.push(label);
            } else if anchor.is_block_terminating() {
                if current_blocks.is_empty() {
                    continue;
                }
                let _ = current_blocks.remove(current_blocks.len() - 1);
            }
        }

        return None;
    }

    pub fn block(&self, label: &Rc<Label>) -> impl Iterator<Item = &Operation> {
        let mut start_idx = None;
        for (i, anchor) in self.anchors.iter().enumerate() {
            if anchor == label {
                start_idx = Some(i + 1);
                break;
            }
        }

        let mut offset = 0usize;
        let mut anchors = start_idx
            .map(|i| &self.anchors[i..])
            .map(IntoIterator::into_iter);

        return core::iter::from_fn(move || {
            let op = anchors.as_mut().and_then(Iterator::next)?;
            match op {
                Operation::Label(_) => offset += 1,
                other if other.is_block_terminating() => match offset.checked_sub(1) {
                    Some(x) => offset = x,
                    None => anchors = None,
                },
                _ => {}
            }

            return Some(op);
        });
    }
}

#[must_use]
#[derive(Debug, Clone)]
pub struct FunctionConfigBuilder {
    pub(crate) inner: FunctionConfig,
    pub(crate) config: Option<(u32, ConfigBuilder)>,
}

impl FunctionConfigBuilder {
    #[doc(hidden)]
    pub unsafe fn __new() -> Self {
        return Self {
            inner: FunctionConfig::default(),
            config: None,
        };
    }

    pub fn param(self, idx: u32) -> ParameterBuilder {
        return ParameterBuilder {
            inner: Parameter::default(),
            function: self,
            idx,
        };
    }

    pub fn add_exec_mode(mut self, exec_mode: ExecutionMode) -> Self {
        self.inner.execution_modes.push(exec_mode);
        self
    }

    pub fn set_entry_point(mut self, exec_model: ExecutionModel) -> Self {
        self.inner.execution_model = Some(exec_model);
        self
    }

    #[inline]
    pub fn build(mut self) -> ConfigBuilder {
        unsafe { self.__build().unwrap_left_unchecked() }
    }

    #[doc(hidden)]
    pub unsafe fn __build(self) -> Either<ConfigBuilder, FunctionConfig> {
        match self.config {
            Some((idx, mut config)) => {
                config.inner.functions.insert(idx, self.inner);
                Either::Left(config)
            }
            None => Either::Right(self.inner),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionConfig {
    #[serde(default)]
    pub execution_model: Option<ExecutionModel>,
    #[serde(default)]
    pub execution_modes: Vec<ExecutionMode>,
    #[serde(default)]
    pub params: VecMap<u32, Parameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionMode {
    Invocations(u32),
    PixelCenterInteger,
    OriginUpperLeft,
    OriginLowerLeft,
    LocalSize(u32, u32, u32),
    LocalSizeHint(u32, u32, u32),
    DepthReplacing,
}

#[must_use]
pub struct ParameterBuilder {
    inner: Parameter,
    idx: u32,
    function: FunctionConfigBuilder,
}

impl ParameterBuilder {
    pub fn set_type(mut self, ty: impl Into<Type>) -> Result<Self> {
        let ty = ty.into();
        self.inner.ty = Some(ty);
        Ok(self)
    }

    pub fn set_kind(mut self, kind: ParameterKind) -> Result<Self> {
        self.inner.kind = kind;
        Ok(self)
    }

    pub fn build(mut self) -> FunctionConfigBuilder {
        self.function.inner.params.insert(self.idx, self.inner);
        self.function
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    #[serde(rename = "type", default)]
    pub ty: Option<Type>,
    pub kind: ParameterKind,
}

impl Parameter {
    pub fn new(ty: impl Into<Option<Type>>, kind: ParameterKind) -> Self {
        return Self {
            ty: ty.into(),
            kind,
        };
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    #[default]
    FunctionParameter,
    Input(u32),
    Output(u32),
    DescriptorSet {
        storage_class: StorageClass,
        set: u32,
        binding: u32,
    },
}

impl ParameterKind {
    pub fn storage_class(&self) -> StorageClass {
        return match self {
            ParameterKind::FunctionParameter => StorageClass::Function,
            ParameterKind::Input(_) => StorageClass::Input,
            ParameterKind::Output(_) => StorageClass::Output,
            ParameterKind::DescriptorSet { storage_class, .. } => *storage_class,
        };
    }
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            ty: Default::default(),
            kind: Default::default(),
        }
    }
}
