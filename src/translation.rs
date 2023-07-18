use crate::{
    ast::{
        function::{ExecutionMode, FunctionBuilder, Schrodinger, Storeable},
        module::{GlobalVariable, ModuleBuilder},
        values::{
            bool::{Bool, BoolSource},
            float::{
                BinarySource as FloatBinarySource, ConstantSource as FloatConstantSource,
                ConversionSource as FloatConversionSource, Float, FloatKind, FloatSource,
                UnarySource as FloatUnarySource,
            },
            integer::{
                BinarySource as IntBinarySource, ConstantSource as IntConstantSource,
                ConversionSource as IntConversionSource, Integer, IntegerKind, IntegerSource,
                UnarySource as IntUnarySource,
            },
            pointer::{Pointer, PointerSource},
            vector::{Vector, VectorSource},
            Value,
        },
        Label, Operation,
    },
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
};
use rspirv::{
    dr::{Module, Operand},
    spirv::{Decoration, ExecutionMode as SpirvExecutionMode, FunctionControl, MemoryAccess, Op},
};
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Constant {
    U32(u32),
    U64(u64),
    F32(u32),
    F64(u64),
    Bool(bool),
}

pub struct Builder {
    inner: rspirv::dr::Builder,
    constants: HashMap<(rspirv::spirv::Word, Constant), rspirv::spirv::Word>,
}

impl Builder {
    pub fn new() -> Self {
        return Self {
            inner: rspirv::dr::Builder::new(),
            constants: HashMap::new(),
        };
    }

    pub fn module(self) -> Module {
        self.inner.module()
    }

    pub fn constant_true(&mut self, result_type: rspirv::spirv::Word) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::Bool(true)))
            .or_insert_with(|| self.inner.constant_true(result_type))
    }

    pub fn constant_false(&mut self, result_type: rspirv::spirv::Word) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::Bool(false)))
            .or_insert_with(|| self.inner.constant_false(result_type))
    }

    pub fn constant_u32(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: u32,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::U32(value)))
            .or_insert_with(|| self.inner.constant_u32(result_type, value))
    }

    pub fn constant_u64(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: u64,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::U64(value)))
            .or_insert_with(|| self.inner.constant_u64(result_type, value))
    }

    pub fn constant_f32(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: f32,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::F32(f32::to_bits(value))))
            .or_insert_with(|| self.inner.constant_f32(result_type, value))
    }

    pub fn constant_f64(
        &mut self,
        result_type: rspirv::spirv::Word,
        value: f64,
    ) -> rspirv::spirv::Word {
        *self
            .constants
            .entry((result_type, Constant::F64(f64::to_bits(value))))
            .or_insert_with(|| self.inner.constant_f64(result_type, value))
    }
}

impl<'a> ModuleBuilder<'a> {
    pub fn translate(self) -> Result<Builder> {
        let mut builder = Builder::new();

        // Capabilities
        for capability in &self.capabilities {
            builder.capability(*capability)
        }

        // TODO extensions

        // TODO extended instruction sets

        // Memory model
        builder.memory_model(self.addressing_model, self.memory_model);

        // TODO entry points

        // TODO debug info

        // TODO anotations

        // Globals
        for global in self.global_variables.iter() {
            let _ = global.translate(&self, &mut builder)?;
        }

        // Function declarations
        for function in self.built_functions.iter() {
            function.function_id.set(Some(builder.id()));
        }

        for function in self.built_functions.iter() {
            function.translate(&self, &mut builder)?;
        }

        // Hidden globals
        for global in self.hidden_global_variables.iter() {
            let _ = global.translate(&self, &mut builder)?;
        }

        return Ok(builder);
    }
}

impl<'a> FunctionBuilder<'a> {
    pub fn translate(&self, module: &ModuleBuilder, builder: &mut Builder) -> Result<()> {
        let return_type = match &self.return_type {
            Some(ty) => ty.clone().translate(module, builder)?,
            None => builder.type_void(),
        };
        let parameters = self
            .parameters
            .iter()
            .cloned()
            .map(|x| x.ty(module).and_then(|x| x.translate(module, builder)))
            .collect::<Result<Vec<_>, _>>()?;

        let function_type = builder.type_function(return_type, parameters);

        // Initialize outsize variables
        for var in self.outside_vars.iter() {
            let _ = var.translate(module, builder)?;
        }

        // Create entry point
        if let Some(ref entry_point) = self.entry_point {
            let function_id = self.function_id.get().ok_or_else(Error::unexpected)?;
            let interface = entry_point
                .interface
                .iter()
                .map(|x| x.translate(module, builder))
                .collect::<Result<Vec<_>>>()?;

            builder.entry_point(
                entry_point.execution_model,
                function_id,
                entry_point.name,
                interface,
            );

            // Add execution mode
            if let Some(ref exec_mode) = entry_point.execution_mode {
                let (execution_mode, params) = match exec_mode {
                    ExecutionMode::Invocations(x) => (SpirvExecutionMode::Invocations, vec![*x]),
                    ExecutionMode::PixelCenterInteger => {
                        (SpirvExecutionMode::PixelCenterInteger, Vec::new())
                    }
                    ExecutionMode::OriginUpperLeft => {
                        (SpirvExecutionMode::OriginUpperLeft, Vec::new())
                    }
                    ExecutionMode::OriginLowerLeft => {
                        (SpirvExecutionMode::OriginLowerLeft, Vec::new())
                    }
                    ExecutionMode::LocalSize(x, y, z) => {
                        (SpirvExecutionMode::LocalSize, vec![*x, *y, *z])
                    }
                    ExecutionMode::LocalSizeHint(x, y, z) => {
                        (SpirvExecutionMode::LocalSizeHint, vec![*x, *y, *z])
                    }
                };
                builder.execution_mode(function_id, execution_mode, params)
            }
        }

        builder.begin_function(
            return_type,
            self.function_id.get(),
            FunctionControl::NONE,
            function_type,
        )?;

        // Initialize function parameters
        for param in self.parameters.iter() {
            let _ = param.translate(module, builder)?;
        }

        builder.begin_block(None)?;

        // Initialize
        for init in self.variable_initializers.iter() {
            let _ = init.translate(module, builder)?;
        }

        // Initialize local variables
        for var in self.local_variables.iter() {
            let _ = var.translate(module, builder)?;
        }

        // Translate anchors
        for anchor in self.anchors.iter() {
            let _ = anchor.translate(module, builder)?;
        }

        builder.end_function()?;
        return Ok(());
    }
}

pub trait Translation {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word>;
}

/* TYPES */
impl Translation for ScalarType {
    fn translate(self, _: &ModuleBuilder, builder: &mut Builder) -> Result<rspirv::spirv::Word> {
        return Ok(match self {
            ScalarType::I32 => builder.type_int(32, 0),
            ScalarType::I64 => builder.type_int(64, 0),
            ScalarType::F32 => builder.type_float(32),
            ScalarType::F64 => builder.type_float(64),
            ScalarType::Bool => builder.type_bool(),
        });
    }
}

impl Translation for CompositeType {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            CompositeType::StructuredArray(elem) => {
                let element = elem.translate(module, builder)?;

                let types_global_values_len = builder.module_ref().types_global_values.len();
                let runtime_array = builder.type_runtime_array(element);
                if builder.module_ref().types_global_values.len() != types_global_values_len {
                    // Add decorators for runtime array
                    builder.decorate(
                        runtime_array,
                        Decoration::ArrayStride,
                        [Operand::LiteralInt32(
                            elem.byte_size().ok_or_else(Error::unexpected)?,
                        )],
                    );
                }

                let types_global_values_len = builder.module_ref().types_global_values.len();
                let structure = builder.type_struct(Some(runtime_array));
                if builder.module_ref().types_global_values.len() != types_global_values_len {
                    // Add decorators for struct
                    builder.decorate(structure, Decoration::Block, None);

                    builder.member_decorate(
                        structure,
                        0,
                        Decoration::Offset,
                        [Operand::LiteralInt32(0)],
                    )
                }

                return Ok(structure);
            }

            CompositeType::Vector(elem, component_count) => {
                let component_type = elem.translate(module, builder)?;
                Ok(builder.type_vector(component_type, component_count))
            }
        }
    }
}

impl Translation for Type {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Type::Pointer(storage_class, pointee) => {
                let pointee_type = pointee.translate(module, builder)?;
                Ok(builder.type_pointer(None, storage_class, pointee_type))
            }
            Type::Scalar(x) => x.translate(module, builder),
            Type::Composite(x) => x.translate(module, builder),
        }
    }
}

/* OPERATIONS */
impl Translation for &GlobalVariable {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            GlobalVariable::Variable(var) => var.translate(module, builder),
            GlobalVariable::Constant(cnst) => cnst.translate(module, builder),
        }
    }
}

impl Translation for &Schrodinger {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self.variable.get() {
            Some(var) => var.translate(module, builder),
            None => todo!(),
        }
    }
}

impl Translation for &Bool {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_bool();
        let res = match &self.source {
            BoolSource::Constant(true) => Ok(builder.constant_true(result_type)),
            BoolSource::Constant(false) => Ok(builder.constant_false(result_type)),
            BoolSource::FromInteger(int) => {
                let operand_1 = int.translate(module, builder)?;
                let zero = match int.kind(module)? {
                    IntegerKind::Short => {
                        let int_type = builder.type_int(32, 0);
                        builder.constant_u32(int_type, 0)
                    }
                    IntegerKind::Long => {
                        let int_type = builder.type_int(64, 0);
                        builder.constant_u64(int_type, 0)
                    }
                };
                builder.i_not_equal(result_type, None, operand_1, zero)
            }
            BoolSource::Negated(x) => {
                let operand = x.translate(module, builder)?;
                builder.logical_not(result_type, None, operand)
            }
            BoolSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Integer {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_int(
            match self.kind(module)? {
                IntegerKind::Short => 32,
                IntegerKind::Long => 64,
            },
            0,
        );

        let res = match &self.source {
            IntegerSource::FunctionParam(_) => builder.function_parameter(result_type),

            IntegerSource::ArrayLength { structured_array } => {
                let structure = structured_array.translate(module, builder)?;
                builder.array_length(result_type, None, structure, 0)
            }

            IntegerSource::Constant(IntConstantSource::Short(x)) => {
                Ok(builder.constant_u32(result_type, *x))
            }

            IntegerSource::Constant(IntConstantSource::Long(x)) => {
                Ok(builder.constant_u64(result_type, *x))
            }

            IntegerSource::Conversion(
                IntConversionSource::FromLong(value)
                | IntConversionSource::FromShort {
                    signed: false,
                    value,
                },
            ) => {
                let unsigned_value = value.translate(module, builder)?;
                builder.u_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromShort {
                signed: true,
                value,
            }) => {
                let unsigned_value = value.translate(module, builder)?;
                builder.s_convert(result_type, None, unsigned_value)
            }

            IntegerSource::Conversion(IntConversionSource::FromPointer(pointer)) => {
                let pointer = pointer.translate(module, builder)?;
                builder.convert_ptr_to_u(result_type, None, pointer)
            }

            IntegerSource::Conversion(IntConversionSource::FromBool(value, kind)) => {
                let (zero, one) = match kind {
                    IntegerKind::Short => (
                        builder.constant_u32(result_type, 0),
                        builder.constant_u32(result_type, 1),
                    ),
                    IntegerKind::Long => (
                        builder.constant_u64(result_type, 0),
                        builder.constant_u64(result_type, 1),
                    ),
                };

                let condition = value.translate(module, builder)?;
                builder.select(result_type, None, condition, one, zero)
            }

            IntegerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }

            IntegerSource::Extracted { vector, index } => {
                let composite = vector.translate(module, builder)?;
                match index.get_constant_value()? {
                    Some(IntConstantSource::Short(x)) => {
                        builder.composite_extract(result_type, None, composite, Some(x))
                    }
                    None => {
                        let index = index.translate(module, builder)?;
                        builder.vector_extract_dynamic(result_type, None, composite, index)
                    }
                    _ => todo!(),
                }
            }

            IntegerSource::FunctionCall {
                function_id, args, ..
            } => {
                let function = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                builder.function_call(result_type, None, function, args)
            }

            IntegerSource::Unary { source, op1 } => {
                let operand = op1.translate(module, builder)?;
                match source {
                    IntUnarySource::Not => builder.not(result_type, None, operand),
                    IntUnarySource::Negate => builder.s_negate(result_type, None, operand),
                }
            }

            IntegerSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, builder)?;
                let operand_2 = op2.translate(module, builder)?;
                match source {
                    IntBinarySource::Add => builder.i_add(result_type, None, operand_1, operand_2),
                    IntBinarySource::Sub => builder.i_sub(result_type, None, operand_1, operand_2),
                    IntBinarySource::Mul => builder.i_mul(result_type, None, operand_1, operand_2),
                    IntBinarySource::SDiv => builder.s_div(result_type, None, operand_1, operand_2),
                    IntBinarySource::UDiv => builder.u_div(result_type, None, operand_1, operand_2),
                    IntBinarySource::SRem => builder.s_rem(result_type, None, operand_1, operand_2),
                    IntBinarySource::URem => builder.u_mod(result_type, None, operand_1, operand_2),
                    IntBinarySource::And => {
                        builder.bitwise_and(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Or => {
                        builder.bitwise_or(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Xor => {
                        builder.bitwise_xor(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::Shl => {
                        builder.shift_left_logical(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::SShr => {
                        builder.shift_right_arithmetic(result_type, None, operand_1, operand_2)
                    }
                    IntBinarySource::UShr => {
                        builder.shift_right_logical(result_type, None, operand_1, operand_2)
                    }
                }
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Float {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = builder.type_float(match self.kind()? {
            FloatKind::Single => 32,
            FloatKind::Double => 64,
        });

        let res = match &self.source {
            FloatSource::FunctionParam(_) => builder.function_parameter(result_type),
            FloatSource::Constant(FloatConstantSource::Single(x)) => {
                Ok(builder.constant_f32(result_type, *x))
            }
            FloatSource::Constant(FloatConstantSource::Double(x)) => {
                Ok(builder.constant_f64(result_type, *x))
            }
            FloatSource::Conversion(
                FloatConversionSource::FromDouble(value) | FloatConversionSource::FromSingle(value),
            ) => {
                let float_value = value.translate(module, builder)?;
                builder.f_convert(result_type, None, float_value)
            }
            FloatSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
            FloatSource::Extracted { vector, index } => {
                let composite = vector.translate(module, builder)?;
                match index.get_constant_value()? {
                    Some(IntConstantSource::Short(x)) => {
                        builder.composite_extract(result_type, None, composite, Some(x))
                    }
                    None => {
                        let index = index.translate(module, builder)?;
                        builder.vector_extract_dynamic(result_type, None, composite, index)
                    }
                    _ => todo!(),
                }
            }
            FloatSource::FunctionCall {
                function_id, args, ..
            } => {
                let function = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                builder.function_call(result_type, None, function, args)
            }
            FloatSource::Unary { source, op1 } => {
                let operand = op1.translate(module, builder)?;
                match source {
                    FloatUnarySource::Negate => builder.f_negate(result_type, None, operand),
                }
            }
            FloatSource::Binary { source, op1, op2 } => {
                let operand_1 = op1.translate(module, builder)?;
                let operand_2 = op2.translate(module, builder)?;
                match source {
                    FloatBinarySource::Add => {
                        builder.f_add(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Sub => {
                        builder.f_sub(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Mul => {
                        builder.f_mul(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Div => {
                        builder.f_div(result_type, None, operand_1, operand_2)
                    }
                    FloatBinarySource::Sqrt => todo!(),
                }
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Rc<Pointer> {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let pointer_type = self.pointer_type().translate(module, builder)?;
        let res = match &self.source {
            PointerSource::FunctionParam => builder.function_parameter(pointer_type),

            PointerSource::Casted { prev } => {
                let prev = prev.translate(module, builder)?;
                builder.bitcast(pointer_type, None, prev)
            }

            PointerSource::FromInteger(value) => {
                let integer_value = value.translate(module, builder)?;
                builder.convert_u_to_ptr(pointer_type, None, integer_value)
            }

            PointerSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(
                    pointer_type,
                    None,
                    pointer,
                    memory_access,
                    additional_params,
                )
            }

            PointerSource::FunctionCall { args } => todo!(),

            PointerSource::Access { base, byte_element } => {
                let base_pointee = &base.pointee;
                let base = base.translate(module, builder)?;

                match base_pointee {
                    Type::Composite(CompositeType::StructuredArray(elem)) => {
                        // Downscale element from byte-sized to element-sized
                        let element_size = Rc::new(Integer::new_constant_usize(
                            elem.byte_size().ok_or_else(Error::unexpected)?,
                            module,
                        ));
                        let element = byte_element
                            .clone()
                            .u_div(element_size, module)?
                            .translate(module, builder)?;

                        let result_type =
                            Type::pointer(self.storage_class, *elem).translate(module, builder)?;
                        let zero =
                            Integer::new_constant_usize(0, module).translate(module, builder)?;

                        builder.access_chain(result_type, None, base, [zero, element])
                    }
                    _ => {
                        let element_size = self
                            .clone()
                            .pointee_byte_size(module)
                            .map(Rc::new)
                            .ok_or_else(|| Error::msg("Pointed element has no size"))?;

                        let element = byte_element
                            .clone()
                            .u_div(element_size, module)?
                            .translate(module, builder)?;

                        builder.ptr_access_chain(pointer_type, None, base, element, None)
                    }
                }
            }

            PointerSource::Variable { init, decorators } => {
                let initializer = init
                    .as_ref()
                    .map(|x| x.translate(module, builder))
                    .transpose()?;
                let variable =
                    builder.variable(pointer_type, None, self.storage_class, initializer);

                decorators
                    .iter()
                    .for_each(|x| x.translate(variable, builder));

                Ok(variable)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Vector {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.translation.get() {
            return Ok(res);
        }

        let result_type = self.vector_type().translate(module, builder)?;
        let res = match &self.source {
            VectorSource::Loaded {
                pointer,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let (memory_access, additional_params) = additional_access_info(*log2_alignment);
                builder.load(result_type, None, pointer, memory_access, additional_params)
            }
        }?;

        self.translation.set(Some(res));
        return Ok(res);
    }
}

impl Translation for &Storeable {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        return match self {
            Storeable::Pointer { pointer, .. } => pointer.translate(module, builder),
            Storeable::Schrodinger(x) => x.translate(module, builder),
        };
    }
}

impl Translation for &Value {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Value::Integer(x) => x.translate(module, builder),
            Value::Float(x) => x.translate(module, builder),
            Value::Pointer(x) => x.translate(module, builder),
            Value::Vector(x) => x.translate(module, builder),
            Value::Bool(x) => x.translate(module, builder),
        }
    }
}

impl Translation for &Label {
    fn translate(self, _: &ModuleBuilder, builder: &mut Builder) -> Result<rspirv::spirv::Word> {
        if let Some(res) = self.0.get() {
            return Ok(res);
        }

        let id = builder.id();
        self.0.set(Some(id));
        return Ok(id);
    }
}

impl Translation for &Operation {
    fn translate(
        self,
        module: &ModuleBuilder,
        builder: &mut Builder,
    ) -> Result<rspirv::spirv::Word> {
        match self {
            Operation::Value(x) => {
                return x.translate(module, builder);
            }

            Operation::Label(x) => {
                let label = x.translate(module, builder)?;
                builder.insert_into_block(
                    rspirv::dr::InsertPoint::End,
                    rspirv::dr::Instruction::new(Op::Label, None, Some(label), Vec::new()),
                )?;
                return Ok(label);
            }

            Operation::Branch(label) => {
                let target_label = label.translate(module, builder)?;
                builder.branch(target_label)
            }

            Operation::BranchConditional {
                condition,
                true_label,
                false_label,
            } => {
                let true_label = true_label.translate(module, builder)?;
                let false_label = false_label.translate(module, builder)?;

                match &condition.source {
                    BoolSource::FromInteger(int) => {
                        let selector = int.translate(module, builder)?;
                        let zero = match int.kind(module)? {
                            IntegerKind::Short => Operand::LiteralInt32(0),
                            IntegerKind::Long => Operand::LiteralInt64(0),
                        };
                        builder.switch(selector, true_label, Some((zero, false_label)))
                    }
                    _ => {
                        let condition = condition.translate(module, builder)?;
                        builder.branch_conditional(condition, true_label, false_label, None)
                    }
                }
            }

            Operation::Store {
                target: pointer,
                value,
                log2_alignment,
            } => {
                let pointer = pointer.translate(module, builder)?;
                let object = value.translate(module, builder)?;
                let (memory_access, additional_params) = log2_alignment
                    .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
                    .unzip();

                builder.store(pointer, object, memory_access, additional_params)
            }

            Operation::FunctionCall { function_id, args } => {
                let function = function_id.get().ok_or_else(Error::unexpected)?;
                let args = args
                    .iter()
                    .map(|x| x.translate(module, builder))
                    .collect::<Result<Vec<_>, _>>()?;

                let void = builder.type_void();
                builder.function_call(void, None, function, args)?;
                Ok(())
            }

            Operation::Nop => builder.nop(),
            Operation::Unreachable => builder.unreachable(),

            Operation::End {
                return_value: Some(value),
            } => {
                let value = value.translate(module, builder)?;
                builder.ret_value(value)
            }
            Operation::End { return_value: None } => builder.ret(),
        }?;

        return Ok(0);
    }
}

impl Deref for Builder {
    type Target = rspirv::dr::Builder;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Builder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

fn additional_access_info(log2_alignment: Option<u32>) -> (Option<MemoryAccess>, Option<Operand>) {
    log2_alignment
        .map(|align| (MemoryAccess::ALIGNED, Operand::LiteralInt32(1 << align)))
        .unzip()
}
