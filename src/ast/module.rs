use super::{
    block::{mvp::translate_constants, translate_block, BlockBuilder, BlockReader},
    function::FunctionBuilder,
    import::{translate_spir_global, ImportResult},
    values::{integer::IntegerKind, pointer::Pointer, Value},
    End,
};
use crate::{
    config::{CapabilityModel, Config, ExtensionModel},
    error::{Error, Result},
    r#type::{ScalarType, Type},
    version::Version,
};
use rspirv::spirv::{AddressingModel, Capability, MemoryModel, StorageClass};
use std::{borrow::Cow, cell::Cell, collections::VecDeque, rc::Rc};
use wasmparser::{ExternalKind, FuncType, Payload, Validator};

#[derive(Debug, Clone)]
pub enum GlobalVariable {
    Variable(Rc<Pointer>),
    Constant(Value),
}

#[derive(Clone)]
pub enum CallableFunction {
    Callback(
        Rc<
            dyn for<'a> Fn(
                &mut BlockBuilder<'a>,
                &mut FunctionBuilder,
                &mut ModuleBuilder,
            ) -> Result<()>,
        >,
    ),
    Defined {
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
        ty: FuncType,
    },
}

impl CallableFunction {
    pub fn callback(
        f: impl 'static
            + for<'a> Fn(
                &mut BlockBuilder<'a>,
                &mut FunctionBuilder,
                &mut ModuleBuilder,
            ) -> Result<()>,
    ) -> Self {
        Self::Callback(Rc::new(f))
    }
}

pub struct ModuleBuilder<'a> {
    pub version: Version,
    pub capabilities: CapabilityModel,
    pub extensions: ExtensionModel,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub wasm_memory64: bool,
    pub functions: Box<[CallableFunction]>,
    pub global_variables: Box<[GlobalVariable]>,
    pub hidden_global_variables: Vec<Rc<Pointer>>,
    pub built_functions: Box<[FunctionBuilder<'a>]>,
}

impl<'a> ModuleBuilder<'a> {
    pub fn new(config: Config, bytes: &'a [u8]) -> Result<Self> {
        let mut validator = Validator::new_with_features(config.features);
        let types = validator.validate_all(bytes)?;

        let wasm_memory64 = match types.memory_count() {
            0 => false,
            _ => types.memory_at(0).memory64,
        };
        let addressing_model = match (config.addressing_model, wasm_memory64) {
            (crate::config::AddressingModel::Logical, _) => AddressingModel::Logical,
            (crate::config::AddressingModel::Physical, false) => AddressingModel::Physical32,
            (crate::config::AddressingModel::Physical, true) => AddressingModel::Physical64,
            (crate::config::AddressingModel::PhysicalStorageBuffer, true) => {
                AddressingModel::PhysicalStorageBuffer64
            }
            _ => return Err(Error::msg("Invalid addressing model")),
        };

        let mut result = Self {
            version: config.version,
            capabilities: config.capabilities,
            extensions: config.extensions,
            memory_model: config.memory_model,
            wasm_memory64,
            addressing_model,
            functions: Box::default(),
            global_variables: Box::default(),
            built_functions: Box::default(),
            hidden_global_variables: Vec::default(),
        };

        let mut functions = Vec::with_capacity(types.function_count() as usize);
        let mut global_variables = Vec::with_capacity(types.global_count() as usize);

        let mut globals = Vec::new();
        let mut code_sections = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        let mut reader = wasmparser::Parser::new(0).parse_all(bytes);
        while let Some(payload) = reader.next().transpose()? {
            match payload {
                Payload::ImportSection(imp) => {
                    imports.reserve(imp.count() as usize);
                    for import in imp.into_iter() {
                        imports.push(import?);
                    }
                }
                Payload::ExportSection(exp) => {
                    exports.reserve(exp.count() as usize);
                    for export in exp.into_iter() {
                        exports.push(export?);
                    }
                }
                Payload::GlobalSection(g) => {
                    globals.reserve(g.count() as usize);
                    for global in g.into_iter() {
                        globals.push(global?);
                    }
                }
                Payload::CodeSectionEntry(body) => code_sections.push(body),
                Payload::End(_) => break,
                _ => continue,
            }
        }

        // Imports
        let mut imported_function_count = 0u32;
        let mut imported_global_count = 0u32;

        for import in imports {
            match import.module {
                "spir_global" => {
                    match translate_spir_global(import.name, import.ty, &mut result)? {
                        Some(ImportResult::Global(var)) => {
                            global_variables.push(var);
                            imported_global_count += 1
                        }
                        Some(ImportResult::Func(f)) => {
                            functions.push(f);
                            imported_function_count += 1
                        }
                        None => todo!(),
                    }
                }
                _ => todo!(),
            }
        }

        // Function definitions
        for i in imported_function_count..types.function_count() {
            let f = match types
                .get(types.function_at(i))
                .ok_or_else(Error::unexpected)?
            {
                wasmparser::types::Type::Sub(ty) => match &ty.structural_type {
                    wasmparser::StructuralType::Func(f) => CallableFunction::Defined {
                        function_id: Rc::new(Cell::new(None)),
                        ty: f.clone(),
                    },
                    _ => return Err(Error::unexpected()),
                },
                _ => return Err(Error::unexpected()),
            };
            functions.push(f);
        }
        result.functions = functions.into_boxed_slice();

        // Global variables
        for i in imported_global_count..types.global_count() {
            let global = types.global_at(i);
            let init_expr = globals
                .get(i as usize)
                .ok_or_else(Error::unexpected)?
                .init_expr;

            let ty = Type::from(global.content_type);
            let mut init_expr_reader = BlockReader::new(init_expr.get_operators_reader());

            let op = init_expr_reader
                .next()
                .transpose()?
                .ok_or_else(Error::element_not_found)?;

            let mut f = FunctionBuilder::default();
            let mut block = translate_block(
                init_expr_reader,
                VecDeque::new(),
                End::Unreachable,
                &mut f,
                &mut result,
            )?;
            translate_constants(&op, &mut block)?;

            let init_value = block
                .stack
                .pop_back()
                .ok_or_else(|| Error::msg("Empty stack"))?;

            global_variables.push(match global.mutable {
                true => GlobalVariable::Variable(Rc::new(Pointer::new_variable_with_init(
                    StorageClass::CrossWorkgroup,
                    ty,
                    init_value,
                    None,
                ))),
                false => GlobalVariable::Constant(init_value),
            })
        }
        result.global_variables = global_variables.into_boxed_slice();

        // Function bodies
        let mut built_functions = Vec::with_capacity(code_sections.len());
        for (i, body) in (imported_function_count..types.function_count()).zip(code_sections) {
            let (function_id, ty) = match result
                .functions
                .get(i as usize)
                .ok_or_else(Error::unexpected)?
            {
                CallableFunction::Defined { function_id, ty } => (function_id.clone(), ty.clone()),
                _ => return Err(Error::unexpected()),
            };

            let config = config
                .functions
                .get(&i)
                .map_or_else(Cow::default, Cow::Borrowed);

            let export = exports
                .iter()
                .find(|x| x.kind == ExternalKind::Func && x.index == i);

            built_functions.push(FunctionBuilder::new(
                function_id,
                export.cloned(),
                &config,
                &ty,
                body,
                &mut result,
            )?);
        }

        result.built_functions = built_functions.into_boxed_slice();
        return Ok(result);
    }

    /// Assert that capability is (or can be) enabled, enabling it if required (and possible).
    pub fn require_capability(&mut self, capability: Capability) -> Result<()> {
        return match self.capabilities {
            CapabilityModel::Static(ref cap) if cap.contains(&capability) => Ok(()),
            CapabilityModel::Dynamic(ref mut cap) => {
                if !cap.contains(&capability) {
                    cap.push(capability)
                }
                Ok(())
            }
            CapabilityModel::Static(_) => {
                return Err(Error::msg(format!(
                    "Capability '{capability:?}' is not enabled"
                )))
            }
        };
    }

    pub fn isize_type(&self) -> ScalarType {
        match self.wasm_memory64 {
            true => ScalarType::I64,
            false => ScalarType::I32,
        }
    }

    pub fn isize_integer_kind(&self) -> IntegerKind {
        match self.wasm_memory64 {
            true => IntegerKind::Long,
            false => IntegerKind::Short,
        }
    }

    pub fn spirv_address_bits(&self, storage_class: StorageClass) -> Option<u32> {
        match (storage_class, self.addressing_model) {
            (_, AddressingModel::Physical32) => Some(32),
            (_, AddressingModel::Physical64)
            | (StorageClass::PhysicalStorageBuffer, AddressingModel::PhysicalStorageBuffer64) => {
                Some(64)
            }
            _ => None,
        }
    }

    pub fn spirv_address_bytes(&self, storage_class: StorageClass) -> Option<u32> {
        self.spirv_address_bits(storage_class).map(|x| x / 8)
    }

    pub fn wasm_address_bits(&self) -> u32 {
        match self.wasm_memory64 {
            true => 64,
            false => 32,
        }
    }

    pub fn wasm_address_bytes(&self) -> u32 {
        self.wasm_address_bits() / 8
    }
}
