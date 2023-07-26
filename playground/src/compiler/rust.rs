use std::process::Stdio;

use super::Compiler;
use crate::tmp::TmpFile;
use color_eyre::Report;
use tokio::io::AsyncWriteExt;
use tracing::error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RustCompiler;

impl Compiler for RustCompiler {
    async fn compile(&self, source: &str) -> Result<Vec<u8>, crate::Error> {
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

        let output = tokio::process::Command::new("rustc")
            .arg(file_name)
            .args([
                "--target",
                "wasm32-unknown-unknown",
                "--out-dir",
                ".",
                "-C",
                "opt-level=s",
            ])
            .stderr(Stdio::piped())
            .current_dir(parent_dir)
            .output()
            .await?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(color_eyre::Report::msg(format!("Compilation error: {message}")).into());
        }

        let target_path = tmp_file.drop_handle().await?;
        let target_wasm_path = target_path.with_extension("wasm");

        let content = tokio::fs::read(&target_wasm_path).await?;
        drop(target_path);

        if let Err(e) = tokio::fs::remove_file(target_wasm_path).await {
            error!("{e}")
        }

        return Ok(content);
    }
}
