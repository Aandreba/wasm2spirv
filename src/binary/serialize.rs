use crate::{
    config::{AddressingModel, CapabilityModel, Config, ExtensionModel, WasmFeatures},
    error::{Error, Result},
    fg::function::{ExecutionMode, FunctionConfig, Parameter, ParameterKind},
    r#type::{CompositeType, ScalarType, Type},
    version::{TargetPlatform, Version},
    Str,
};
use spirv::{Capability, ExecutionModel, MemoryModel, StorageClass};
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};
use vector_mapp::vec::VecMap;

pub trait BinarySerialize {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()>;
}

/* SERIALIZE */
impl BinarySerialize for str {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        <[u8]>::serialize_into(self.as_bytes(), writer)
    }
}

impl BinarySerialize for String {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        str::serialize_into(self, writer)
    }
}

impl<'a> BinarySerialize for Str<'a> {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        str::serialize_into(self, writer)
    }
}

impl BinarySerialize for bool {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        (*self as u8).serialize_into(writer)
    }
}

impl BinarySerialize for u8 {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(*self)?;
        Ok(())
    }
}

impl BinarySerialize for u32 {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self)?;
        Ok(())
    }
}

impl BinarySerialize for WasmFeatures {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u64(self.into_integer())?;
        Ok(())
    }
}

impl BinarySerialize for Version {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u8(self.major)?;
        writer.write_u8(self.minor)?;
        return Ok(());
    }
}

impl BinarySerialize for TargetPlatform {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            TargetPlatform::Vulkan(version) => {
                writer.write_u16(0)?;
                version.serialize_into(writer)?;
            }
            TargetPlatform::Universal(version) => {
                writer.write_u16(1)?;
                version.serialize_into(writer)?;
            }
        };

        return Ok(());
    }
}

impl BinarySerialize for AddressingModel {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16(*self as u16)?;
        Ok(())
    }
}

impl BinarySerialize for MemoryModel {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinarySerialize for Capability {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinarySerialize for ExecutionModel {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinarySerialize for StorageClass {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u32(*self as u32)?;
        Ok(())
    }
}

impl BinarySerialize for CapabilityModel {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            CapabilityModel::Static(data) => {
                writer.write_u8(0)?;
                data.serialize_into(writer)?;
            }
            CapabilityModel::Dynamic(data) => {
                writer.write_u8(1)?;
                data.serialize_into(writer)?;
            }
        };

        Ok(())
    }
}

impl BinarySerialize for ExtensionModel {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ExtensionModel::Static(data) => {
                writer.write_u8(0)?;
                data.serialize_into(writer)?;
            }
            ExtensionModel::Dynamic(data) => {
                writer.write_u8(1)?;
                data.serialize_into(writer)?;
            }
        };

        Ok(())
    }
}

impl BinarySerialize for ExecutionMode {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ExecutionMode::Invocations(x) => {
                writer.write_u16(0)?;
                writer.write_u32(*x)?;
            }
            ExecutionMode::PixelCenterInteger => writer.write_u16(1)?,
            ExecutionMode::OriginUpperLeft => writer.write_u16(2)?,
            ExecutionMode::OriginLowerLeft => writer.write_u16(3)?,
            ExecutionMode::LocalSize(x, y, z) => {
                writer.write_u16(4)?;
                writer.write_u32(*x)?;
                writer.write_u32(*y)?;
                writer.write_u32(*z)?;
            }
            ExecutionMode::LocalSizeHint(x, y, z) => {
                writer.write_u16(5)?;
                writer.write_u32(*x)?;
                writer.write_u32(*y)?;
                writer.write_u32(*z)?;
            }
        }

        Ok(())
    }
}

impl BinarySerialize for ScalarType {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_u16(*self as u16)?;
        Ok(())
    }
}

impl BinarySerialize for CompositeType {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            CompositeType::Structured(elem) => {
                writer.write_u16(0)?;
                elem.serialize_into(writer)?;
            }
            CompositeType::StructuredArray(elem) => {
                writer.write_u16(1)?;
                elem.serialize_into(writer)?;
            }
            CompositeType::Vector(elem, count) => {
                writer.write_u16(2)?;
                elem.serialize_into(writer)?;
                writer.write_u32(*count)?;
            }
        };

        Ok(())
    }
}

impl BinarySerialize for Type {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Type::Pointer(sc, pointee) => {
                writer.write_u16(0)?;
                sc.serialize_into(writer)?;
                pointee.serialize_into(writer)?;
            }
            Type::Scalar(x) => {
                writer.write_u16(1)?;
                x.serialize_into(writer)?;
            }
            Type::Composite(x) => {
                writer.write_u16(2)?;
                x.serialize_into(writer)?;
            }
        };

        Ok(())
    }
}

impl BinarySerialize for ParameterKind {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            ParameterKind::FunctionParameter => writer.write_u16(0)?,
            ParameterKind::Input => writer.write_u16(1)?,
            ParameterKind::Output => writer.write_u16(2)?,
            ParameterKind::DescriptorSet {
                storage_class,
                set,
                binding,
            } => {
                writer.write_u16(3)?;
                storage_class.serialize_into(writer)?;
                writer.write_u32(*set)?;
                writer.write_u32(*binding)?;
            }
        };

        Ok(())
    }
}

impl BinarySerialize for Parameter {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.ty.serialize_into(writer)?;
        self.kind.serialize_into(writer)?;
        self.is_extern_pointer.serialize_into(writer)?;
        Ok(())
    }
}

impl BinarySerialize for FunctionConfig {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.execution_model.serialize_into(writer)?;
        self.execution_mode.serialize_into(writer)?;
        self.params.serialize_into(writer)?;
        Ok(())
    }
}

impl BinarySerialize for Config {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        self.platform.serialize_into(writer)?;
        self.features.serialize_into(writer)?;
        self.addressing_model.serialize_into(writer)?;
        self.memory_model.serialize_into(writer)?;
        self.capabilities.serialize_into(writer)?;
        self.extensions.serialize_into(writer)?;
        self.functions.serialize_into(writer)?;
        Ok(())
    }
}

// BLANKETS
impl<K: BinarySerialize, V: BinarySerialize> BinarySerialize for HashMap<K, V> {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.serialize_into(writer)?;
            value.serialize_into(writer)?;
        }

        Ok(())
    }
}

impl<K: BinarySerialize, V: BinarySerialize> BinarySerialize for BTreeMap<K, V> {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.serialize_into(writer)?;
            value.serialize_into(writer)?;
        }

        Ok(())
    }
}

impl<K: BinarySerialize, V: BinarySerialize> BinarySerialize for VecMap<K, V> {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for (key, value) in self {
            key.serialize_into(writer)?;
            value.serialize_into(writer)?;
        }

        Ok(())
    }
}

impl<T: BinarySerialize> BinarySerialize for Option<T> {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Some(val) => {
                writer.write_u8(1)?;
                val.serialize_into(writer)?;
            }
            None => writer.write_u8(0)?,
        };

        Ok(())
    }
}

impl<T: BinarySerialize> BinarySerialize for [T] {
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        let len = u32::try_from(self.len()).map_err(|e| Error::msg(e.to_string()))?;

        writer.write_u32(len)?;
        for element in self {
            element.serialize_into(writer)?;
        }

        Ok(())
    }
}

impl<T: BinarySerialize> BinarySerialize for Vec<T> {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        <[T]>::serialize_into(self, writer)
    }
}

impl<T: ?Sized + BinarySerialize> BinarySerialize for Box<T> {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        T::serialize_into(self, writer)
    }
}

impl<T: ?Sized + BinarySerialize> BinarySerialize for Rc<T> {
    #[inline]
    fn serialize_into<W: ?Sized + std::io::Write>(&self, writer: &mut W) -> Result<()> {
        T::serialize_into(self, writer)
    }
}

pub(crate) trait WriteLe: std::io::Write {
    fn write_i8(&mut self, value: i8) -> std::io::Result<()> {
        self.write_all(&i8::to_le_bytes(value))
    }

    fn write_u8(&mut self, value: u8) -> std::io::Result<()> {
        self.write_all(&u8::to_le_bytes(value))
    }

    fn write_i16(&mut self, value: i16) -> std::io::Result<()> {
        self.write_all(&i16::to_le_bytes(value))
    }

    fn write_u16(&mut self, value: u16) -> std::io::Result<()> {
        self.write_all(&u16::to_le_bytes(value))
    }

    fn write_i32(&mut self, value: i32) -> std::io::Result<()> {
        self.write_all(&i32::to_le_bytes(value))
    }

    fn write_u32(&mut self, value: u32) -> std::io::Result<()> {
        self.write_all(&u32::to_le_bytes(value))
    }

    fn write_i64(&mut self, value: i64) -> std::io::Result<()> {
        self.write_all(&i64::to_le_bytes(value))
    }

    fn write_u64(&mut self, value: u64) -> std::io::Result<()> {
        self.write_all(&u64::to_le_bytes(value))
    }

    fn write_i128(&mut self, value: i128) -> std::io::Result<()> {
        self.write_all(&i128::to_le_bytes(value))
    }

    fn write_u128(&mut self, value: u128) -> std::io::Result<()> {
        self.write_all(&u128::to_le_bytes(value))
    }

    fn write_f32(&mut self, value: f32) -> std::io::Result<()> {
        self.write_all(&f32::to_le_bytes(value))
    }

    fn write_f64(&mut self, value: f64) -> std::io::Result<()> {
        self.write_all(&f64::to_le_bytes(value))
    }
}

impl<T: ?Sized + std::io::Write> WriteLe for T {}
