use super::CompilerError;
use crate::{
    error::{Error, Result},
    Compilation,
};
use naga::{back::glsl::PipelineOptions, proc::BoundsCheckPolicies, valid};
use rspirv::dr::Operand;
use spirv::{ExecutionModel, Op};

macro_rules! tri {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Err(e.into())),
        }
    };
}

impl Compilation {
    #[cfg(feature = "naga-glsl")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))))]
    pub fn glsl(&self) -> Result<&str> {
        use naga::back::glsl;

        match self.glsl.get_or_try_init(|| {
            let (exec_model, name) = self.naga_info()?;
            let pipeline_options = PipelineOptions {
                shader_stage: match exec_model {
                    ExecutionModel::Vertex => naga::ShaderStage::Vertex,
                    ExecutionModel::Fragment => naga::ShaderStage::Fragment,
                    ExecutionModel::GLCompute => naga::ShaderStage::Compute,
                    other => {
                        return Err(Error::msg(format!(
                            "Unsupported execution model '{other:?}'"
                        )))
                    }
                },
                entry_point: name.to_string(),
                multiview: None,
            };

            let module = tri!(self.naga_module()?);
            let info = tri!(valid::Validator::new(
                valid::ValidationFlags::empty(),
                valid::Capabilities::all()
            )
            .validate(&module));

            let version = match 0 {
                _ => naga::back::glsl::Version::Desktop(450),
            };

            let options = glsl::Options {
                version,
                ..Default::default()
            };

            let mut result = String::new();
            let mut writer = tri!(naga::back::glsl::Writer::new(
                &mut result,
                &module,
                &info,
                &options,
                &pipeline_options,
                BoundsCheckPolicies::default(),
            ));

            tri!(writer.write());
            Ok::<_, Error>(Ok(result.into_boxed_str()))
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    fn naga_module(&self) -> Result<Result<naga::Module, CompilerError>> {
        let module = tri!(naga::front::spv::parse_u8_slice(
            self.bytes()?,
            &naga::front::spv::Options::default()
        ));
        return Ok(Ok(module));
    }

    fn naga_info(&self) -> Result<(ExecutionModel, &str)> {
        let module = self.module()?;
        if module.entry_points.len() != 1 {
            return Err(Error::msg("Exactly one entry point must be specified"));
        }

        let entry_point = &module.entry_points[0];
        debug_assert_eq!(entry_point.class.opcode, Op::EntryPoint);

        let execution_model = match entry_point.operands.get(0) {
            Some(Operand::ExecutionModel(model)) => model,
            _ => return Err(Error::unexpected()),
        };

        let name = match entry_point.operands.get(2) {
            Some(Operand::LiteralString(mode)) => mode,
            _ => return Err(Error::unexpected()),
        };

        Ok((*execution_model, name))
    }
}
