use super::{
    block::{translate_block, BlockBuilder, BlockReader},
    module::ModuleBuilder,
    values::{integer::Integer, pointer::Pointer, Value},
    End, Label, Operation,
};
use crate::{
    config::{execution_model_capabilities, storage_class_capabilities, ConfigBuilder},
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::Type,
    version::Version,
};
use once_cell::unsync::OnceCell;
use rspirv::spirv::{Capability, ExecutionModel, StorageClass};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, cell::Cell, collections::VecDeque, rc::Rc};
use vector_mapp::vec::VecMap;
use wasmparser::{Export, FuncType, FunctionBody, ValType};

/// May be a pointer or an integer, but you won't know until you try to store into it.
#[derive(Debug, Clone)]
pub struct Schrodinger {
    pub variable: OnceCell<Rc<Pointer>>,
    pub offset: OnceCell<Rc<Pointer>>,
}

impl Schrodinger {
    fn offset_variable(&self, module: &ModuleBuilder) -> &Rc<Pointer> {
        self.offset.get_or_init(|| {
            let init = Rc::new(Integer::new_constant_usize(0, module));
            Rc::new(Pointer::new_variable_with_init(
                StorageClass::Function,
                module.isize_type(),
                init,
                None,
            ))
        })
    }

    pub fn store_integer(
        &self,
        value: Rc<Integer>,
        block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<(Operation, Option<Operation>)> {
        let variable = self.variable.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                StorageClass::Function,
                module.isize_type(),
                None,
            ))
        });

        let offset = if let Some(sch_offset) = self.offset.get() {
            let zero = Rc::new(Integer::new_constant_usize(0, module));
            Some(sch_offset.clone().store(zero, None, block, module)?)
        } else {
            None
        };

        let value = match &variable.pointee {
            Type::Scalar(x) if x == &module.isize_type() => {
                variable.clone().store(value, None, block, module)
            }
            Type::Pointer(storage_class, pointee) => {
                let value = value.to_pointer(*storage_class, Type::clone(pointee), module)?;
                variable.clone().store(value, None, block, module)
            }
            _ => return Err(Error::unexpected()),
        }?;

        Ok((value, offset))
    }

    pub fn store_pointer(
        &self,
        value: Rc<Pointer>,
        block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<(Operation, Option<Operation>)> {
        let (value, offset) = value.split_ptr_offset(module)?;

        let variable = self.variable.get_or_init(|| {
            Rc::new(Pointer::new_variable(
                StorageClass::Function,
                Type::pointer(value.storage_class, value.pointee.clone()),
                None,
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

        let value = match &variable.pointee {
            Type::Scalar(x) if x == &module.isize_type() => {
                let value = value.to_integer(module)?;
                variable.clone().store(value, None, block, module)
            }
            Type::Pointer(sch_storage_class, pointee)
                if sch_storage_class == &value.storage_class =>
            {
                let value = value.cast(Type::clone(pointee));
                variable.clone().store(value, None, block, module)
            }
            _ => return Err(Error::unexpected()),
        }?;

        Ok((value, offset))
    }

    pub fn load(&self, block: &mut BlockBuilder, module: &mut ModuleBuilder) -> Result<Value> {
        let variable = self
            .variable
            .get()
            .ok_or_else(|| Error::msg("Schrodinger variable is still uninitialized"))?;

        let mut value = variable.clone().load(None, block, module)?;
        if let Some(offset) = self.offset.get() {
            let offset = offset.clone().load(None, block, module)?.into_integer()?;
            value = value.i_add(offset, module)?;
        }

        return Ok(value);
    }
}

#[derive(Debug, Clone)]
pub enum Storeable {
    Pointer(Rc<Pointer>),
    Schrodinger(Rc<Schrodinger>),
}

#[derive(Debug, Clone)]
pub struct EntryPoint<'a> {
    pub execution_model: ExecutionModel,
    pub execution_mode: Option<ExecutionMode>,
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

            let ty = param.ty.clone().unwrap_or_else(|| Type::from(*wasm_ty));
            let storage_class = param.kind.storage_class();

            let variable = match param.kind {
                ParameterKind::FunctionParameter => {
                    let param = Value::function_parameter(ty.clone());
                    let var = Rc::new(Pointer::new_variable(storage_class, ty, Vec::new()));
                    variable_initializers.push(var.clone().store(
                        param.clone(),
                        None,
                        &mut BlockBuilder::dummy(),
                        module,
                    )?);
                    params.push(param);
                    var
                }

                ParameterKind::Input(location) => {
                    let mut decorators = vec![VariableDecorator::Location(location)];
                    match ty {
                        Type::Scalar(_) => decorators.push(VariableDecorator::Flat),
                        _ => {}
                    };

                    let param =
                        Rc::new(Pointer::new_variable(storage_class, ty.clone(), decorators));
                    outside_vars.push(param.clone());
                    interface.push(param.clone());

                    let variable = Rc::new(Pointer::new_variable(
                        StorageClass::Function,
                        ty,
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
                    Rc::new(Pointer::new_variable(storage_class, ty, decorators))
                }

                ParameterKind::DescriptorSet { set, binding, .. } => {
                    Rc::new(Pointer::new_variable(
                        storage_class,
                        ty,
                        vec![
                            VariableDecorator::DesctiptorSet(set),
                            VariableDecorator::Binding(binding),
                        ],
                    ))
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
                pointer: variable,
                is_extern_pointer: param.is_extern_pointer,
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
                        offset: OnceCell::new(),
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

        let entry_point = match (export, config.execution_model) {
            (Some(export), Some(execution_model)) => Some(EntryPoint {
                execution_model,
                execution_mode: config.execution_mode.clone(),
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
#[derive(Debug)]
pub struct FunctionConfigBuilder<'a> {
    pub(crate) inner: FunctionConfig,
    pub(crate) idx: u32,
    pub(crate) config: &'a mut ConfigBuilder,
}

impl<'a> FunctionConfigBuilder<'a> {
    pub fn param(self, idx: u32) -> ParameterBuilder<'a> {
        return ParameterBuilder {
            inner: Parameter::default(),
            function: self,
            idx,
        };
    }

    pub fn set_exec_mode(mut self, exec_mode: ExecutionMode) -> Result<Self> {
        if let Some(capability) = exec_mode.required_capability() {
            self.config.require_capability(capability)?;
        }
        self.inner.execution_mode = Some(exec_mode);
        Ok(self)
    }

    pub fn set_entry_point(mut self, exec_model: ExecutionModel) -> Result<Self> {
        let capability = match exec_model {
            ExecutionModel::Vertex | ExecutionModel::Fragment | ExecutionModel::GLCompute => {
                Capability::Shader
            }
            ExecutionModel::TessellationEvaluation | ExecutionModel::TessellationControl => {
                Capability::Tessellation
            }
            ExecutionModel::Geometry => Capability::Geometry,
            ExecutionModel::Kernel => Capability::Kernel,
            _ => todo!(),
        };

        self.config.require_capability(capability)?;
        self.inner.execution_model = Some(exec_model);
        Ok(self)
    }

    pub fn build(self) -> &'a mut ConfigBuilder {
        self.config.inner.functions.insert(self.idx, self.inner);
        self.config
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionConfig {
    #[serde(default)]
    pub execution_model: Option<ExecutionModel>,
    #[serde(default)]
    pub execution_mode: Option<ExecutionMode>,
    #[serde(default)]
    pub params: VecMap<u32, Parameter>,
}

impl FunctionConfig {
    pub fn required_capabilities(&self) -> Vec<Capability> {
        let mut res = Vec::new();

        if let Some(execution_model) = self.execution_model {
            res.extend(execution_model_capabilities(execution_model));
        }

        if let Some(execution_mode) = &self.execution_mode {
            res.extend(execution_mode.required_capability());
        }

        res.extend(self.params.values().flat_map(|x| x.required_capabilities()));

        res
    }
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
}

impl ExecutionMode {
    pub fn required_capability(&self) -> Option<Capability> {
        return Some(match self {
            Self::Invocations(_) => Capability::Geometry,
            Self::PixelCenterInteger | Self::OriginUpperLeft | Self::OriginLowerLeft => {
                Capability::Shader
            }
            Self::LocalSizeHint(_, _, _) => Capability::Kernel,
            _ => return None,
        });
    }
}

#[must_use]
pub struct ParameterBuilder<'a> {
    inner: Parameter,
    idx: u32,
    function: FunctionConfigBuilder<'a>,
}

impl<'a> ParameterBuilder<'a> {
    /// This will determine wether tha pointer itself, instead of it's pointed value, will be the one pushed to,
    /// and poped from, the stack.
    pub fn set_extern_pointer(mut self, extern_pointer: bool) -> Self {
        self.inner.is_extern_pointer = extern_pointer;
        self
    }

    pub fn set_type(mut self, ty: impl Into<Type>) -> Result<Self> {
        let ty = ty.into();

        for capability in ty.required_capabilities() {
            self.function.config.require_capability(capability)?;
        }

        self.inner.ty = Some(ty);
        Ok(self)
    }

    pub fn set_kind(mut self, kind: ParameterKind) -> Result<Self> {
        for capability in kind.required_capabilities() {
            self.function.config.require_capability(capability)?;
        }

        self.inner.kind = kind;
        Ok(self)
    }

    pub fn build(mut self) -> FunctionConfigBuilder<'a> {
        self.function.inner.params.insert(self.idx, self.inner);
        self.function
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    #[serde(rename = "type", default)]
    pub ty: Option<Type>,
    pub kind: ParameterKind,
    /// This will determine wether tha pointer itself, instead of it's pointed value, will be the one pushed to,
    /// and poped from, the stack.
    #[serde(default)]
    pub is_extern_pointer: bool,
}

impl Parameter {
    pub fn new(ty: impl Into<Option<Type>>, kind: ParameterKind, is_extern_pointer: bool) -> Self {
        return Self {
            ty: ty.into(),
            kind,
            is_extern_pointer,
        };
    }

    pub fn required_capabilities(&self) -> Vec<Capability> {
        let mut res = Vec::new();

        if let Some(ty) = &self.ty {
            res.extend(ty.required_capabilities());
        }

        res.extend(self.kind.required_capabilities());

        res
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
    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            ParameterKind::FunctionParameter | ParameterKind::Input(_) => Vec::new(),
            ParameterKind::Output(_) => vec![Capability::Shader],
            ParameterKind::DescriptorSet { storage_class, .. } => {
                let mut res = vec![Capability::Shader];
                res.extend(storage_class_capabilities(*storage_class));
                res
            }
        }
    }

    pub fn storage_class(&self) -> StorageClass {
        match self {
            ParameterKind::FunctionParameter => StorageClass::Function,
            ParameterKind::Input(_) => StorageClass::Input,
            ParameterKind::Output(_) => StorageClass::Output,
            ParameterKind::DescriptorSet { storage_class, .. } => *storage_class,
        }
    }
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            ty: Default::default(),
            kind: Default::default(),
            is_extern_pointer: false,
        }
    }
}
