use super::Compiler;
use crate::tmp::TmpFile;
use color_eyre::Report;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RustCompiler;

impl Compiler for RustCompiler {
    async fn compile(source: &str) -> color_eyre::Result<Vec<u8>> {
        let mut tmp_file = TmpFile::new().await?;
        tmp_file.write_all(source.as_bytes()).await?;

        let file_name = tmp_file
            .path()
            .file_name()
            .ok_or_else(|| Report::msg("File name not found"))?;

        let parent_dir = tmp_file
            .path()
            .parent()
            .ok_or_else(|| Report::msg("Parent directory not found"))?;

        tokio::process::Command::new("rustc")
            .arg(file_name)
            .args([
                "--target",
                "wasm32-unknown-unknown",
                "--out-dir",
                ".",
                "-C",
                "opt-level=s",
            ])
            .current_dir(parent_dir)
            .status()
            .await?
            .exit_ok()?;

        let target_name = tmp_file.path().with_extension(".wasm");
        drop(tmp_file);
        return tokio::fs::read(target_name).await.map_err(Into::into);
    }
}
