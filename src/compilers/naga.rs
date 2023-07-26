use crate::{
    error::{Error, Result},
    Compilation,
};
use docfg::docfg;
use naga::{proc::BoundsCheckPolicies, valid};
use rspirv::dr::Operand;
use spirv::{ExecutionModel, Op};

impl Compilation {
    #[docfg(feature = "naga-validate")]
    pub fn naga_validate(&self) -> Result<()> {
        let _ = self.naga_module()?;
        return Ok(());
    }

    #[docfg(feature = "naga-glsl")]
    pub fn naga_glsl(&self) -> Result<String> {
        use naga::back::glsl;

        tracing::warn!("GLSL is currently on secondary support for naga.");
        let (exec_model, name) = self.naga_info()?;
        let (module, info) = self.naga_module()?;

        let pipeline_options = glsl::PipelineOptions {
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
            entry_point: name.into(),
            multiview: None,
        };

        let version = match 0 {
            // TODO
            _ => glsl::Version::Desktop(450),
        };

        let options = glsl::Options {
            version,
            ..Default::default()
        };

        let mut result = String::new();
        let mut writer = glsl::Writer::new(
            &mut result,
            &module,
            &info,
            &options,
            &pipeline_options,
            BoundsCheckPolicies::default(),
        )?;

        writer.write()?;
        return Ok(result);
    }

    #[docfg(feature = "naga-hlsl")]
    pub fn naga_hlsl(&self) -> Result<String> {
        use naga::back::hlsl;

        let (module, info) = self.naga_module()?;
        let options = hlsl::Options::default();

        let mut result = String::new();
        let mut writer = hlsl::Writer::new(&mut result, &options);

        writer.write(&module, &info)?;
        return Ok(result);
    }

    #[docfg(feature = "naga-msl")]
    pub fn naga_msl(&self) -> Result<String> {
        use naga::back::msl;

        let (module, info) = self.naga_module()?;
        let pipeline_options = msl::PipelineOptions::default();
        let options = msl::Options::default();

        let mut writer = msl::Writer::new(String::new());
        writer.write(&module, &info, &options, &pipeline_options)?;
        return Ok(writer.finish());
    }

    #[docfg(feature = "naga-wgsl")]
    pub fn naga_wgsl(&self) -> Result<String> {
        use naga::back::wgsl;

        tracing::warn!("WGSL is currently on secondary support for naga.");
        let (module, info) = self.naga_module()?;

        let mut writer = wgsl::Writer::new(String::new(), wgsl::WriterFlags::EXPLICIT_TYPES);
        writer.write(&module, &info)?;
        return Ok(writer.finish());
    }

    fn naga_module(&self) -> Result<&(naga::Module, naga::valid::ModuleInfo)> {
        match self.naga_module.get_or_try_init(|| {
            let options = &naga::front::spv::Options::default();
            let module =
                naga::front::spv::Frontend::new(self.words()?.iter().copied(), options).parse()?;

            let info =
                valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::all())
                    .validate(&module)?;

            return Ok::<_, Error>(Ok((module, info)));
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
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
