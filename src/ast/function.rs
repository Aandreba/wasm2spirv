use super::{
    block::{translate_function, BlockReader},
    module::ModuleBuilder,
    values::{integer::Integer, pointer::Pointer, Value},
    Operation,
};
use crate::{
    config::{storage_class_capability, ConfigBuilder},
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::Type,
};
use once_cell::unsync::OnceCell;
use rspirv::spirv::{Capability, ExecutionModel, StorageClass};
use std::{borrow::Cow, cell::Cell, collections::BTreeMap, rc::Rc};
use wasmparser::{Export, FuncType, FunctionBody, ValType};

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

#[derive(Debug, Clone)]
pub struct EntryPoint<'a> {
    pub execution_model: ExecutionModel,
    pub name: &'a str,
    pub interface: Box<[Rc<Pointer>]>,
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
    pub outside_vars: Box<[Rc<Pointer>]>,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(
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
        let return_type = ty.results().get(0).cloned().map(Type::from);

        // Add function params as local variables
        for (wasm_ty, i) in ty.params().iter().zip(0..) {
            let param = config
                .params
                .get(&i)
                .map_or_else(Cow::default, Cow::Borrowed);

            let ty = param.ty.clone().unwrap_or_else(|| Type::from(*wasm_ty));
            let storage_class = param.kind.storage_class();

            let decorators = match param.kind {
                ParameterKind::FunctionParameter => {
                    params.push(Value::func);
                    Vec::new()
                }
                ParameterKind::Input | ParameterKind::Output => Vec::new(),
                ParameterKind::DescriptorSet { set, binding, .. } => vec![
                    VariableDecorator::DesctiptorSet(set),
                    VariableDecorator::Binding(binding),
                ],
            };

            let variable = Rc::new(Pointer::new_variable(storage_class, ty, decorators));

            if storage_class != StorageClass::Function {
                outside_vars.push(variable.clone())
            }

            if matches!(storage_class, StorageClass::Input | StorageClass::Output) {
                interface.push(variable.clone())
            }

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

        let entry_point = match (export, config.entry_point_exec_model) {
            (Some(export), Some(execution_model)) => Some(EntryPoint {
                execution_model,
                name: export.name,
                interface: interface.into_boxed_slice(), // TODO
            }),
            (None, Some(_)) => todo!(),
            _ => None,
        };

        let mut result = Self {
            function_id: Rc::new(Cell::new(None)),
            anchors: Vec::new(),
            parameters: params.into_boxed_slice(),
            local_variables: locals.into_boxed_slice(),
            outside_vars: outside_vars.into_boxed_slice(),
            entry_point,
            return_type,
        };

        let reader = BlockReader::new(body.get_operators_reader()?);
        translate_function(reader, result.return_type.clone(), &mut result, module)?;

        return Ok(result);
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
    pub fn set_param(self, idx: u32) -> ParameterBuilder<'a> {
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
        self.inner.exec_mode = Some(exec_mode);
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
        self.inner.entry_point_exec_model = Some(exec_model);
        Ok(self)
    }

    pub fn build(self) -> &'a mut ConfigBuilder {
        self.config.inner.functions.insert(self.idx, self.inner);
        self.config
    }
}

#[derive(Debug, Clone, Default)]
pub struct FunctionConfig {
    pub entry_point_exec_model: Option<ExecutionModel>,
    pub exec_mode: Option<ExecutionMode>,
    pub params: BTreeMap<u32, Parameter>,
}

#[derive(Debug, Clone)]
pub enum ExecutionMode {
    Invocations(u32),
    PixelCenterInteger,
    OriginUpperLeft,
    OriginLowerLeft,
    LocalSize { x: u32, y: u32, z: u32 },
    LocalSizeHint { x: u32, y: u32, z: u32 },
}

impl ExecutionMode {
    pub fn required_capability(&self) -> Option<Capability> {
        return Some(match self {
            Self::Invocations(_) => Capability::Geometry,
            Self::PixelCenterInteger | Self::OriginUpperLeft | Self::OriginLowerLeft => {
                Capability::Shader
            }
            Self::LocalSizeHint { .. } => Capability::Kernel,
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

#[derive(Debug, Clone)]
pub struct Parameter {
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

#[derive(Debug, Clone, Default)]
pub enum ParameterKind {
    #[default]
    FunctionParameter,
    Input,
    Output,
    DescriptorSet {
        storage_class: StorageClass,
        set: u32,
        binding: u32,
    },
}

impl ParameterKind {
    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            ParameterKind::FunctionParameter | ParameterKind::Input => Vec::new(),
            ParameterKind::Output => vec![Capability::Shader],
            ParameterKind::DescriptorSet { storage_class, .. } => {
                let mut res = vec![Capability::Shader];
                res.extend(storage_class_capability(*storage_class));
                res
            }
        }
    }

    pub fn storage_class(&self) -> StorageClass {
        match self {
            ParameterKind::FunctionParameter => StorageClass::Function,
            ParameterKind::Input => StorageClass::Input,
            ParameterKind::Output => StorageClass::Output,
            ParameterKind::DescriptorSet { storage_class, .. } => *storage_class,
        }
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
