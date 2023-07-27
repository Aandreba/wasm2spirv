use super::Compiler;
use crate::tmp::{TmpFile, TmpPath};
use color_eyre::Report;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RustCompiler;

impl Compiler for RustCompiler {
    async fn compile(&self, source: &str) -> Result<Vec<u8>, crate::Error> {
        let source = format!("#![no_std]\n#[panic_handler]\nfn panic(_:&core::panic::PanicInfo) -> ! {{ loop {{}} }}{source}");

        let mut tmp_file = TmpFile::new("rs").await?;
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
                "--crate-type",
                "cdylib",
                "-C",
                "opt-level=s",
                "--target",
                "wasm32-unknown-unknown",
                "--out-dir",
                ".",
            ])
            .kill_on_drop(true)
            .stderr(Stdio::piped())
            .current_dir(parent_dir)
            .output()
            .await?;

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(color_eyre::Report::msg(message.into_owned()).into());
        }

        let target_path = tmp_file.drop_handle().await?;
        let target_wasm_path = TmpPath::from(target_path.with_extension("wasm"));
        let content = tokio::fs::read(&target_wasm_path).await?;

        drop(target_wasm_path);
        return Ok(content);
    }
}
