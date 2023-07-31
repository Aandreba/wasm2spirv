use super::Compiler;
use crate::tmp::{TmpFile, TmpPath};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ZigCompiler;

impl Compiler for ZigCompiler {
    async fn compile(&self, source: &str) -> Result<Vec<u8>, crate::Error> {
        let mut tmp_file = TmpFile::new("zig").await?;
        tmp_file.write_all(source.as_bytes()).await?;

        let target_path = tmp_file.drop_handle().await?;
        let target_wasm_path = TmpPath::from(target_path.with_extension("wasm"));

        // zig build-lib examples/{{TEST}}/{{TEST}}.zig -target wasm32-freestanding -O ReleaseSmall -femit-bin=examples/out/{{TEST}}.wasm -dynamic -rdynamic
        let output = tokio::process::Command::new("zig")
            .arg("build-lib")
            .arg(&target_path)
            .args([
                "-target",
                "wasm32-freestanding",
                "-O",
                "ReleaseSmall",
                "-dynamic",
                "-rdynamic",
            ])
            .arg(format!("-femit-bin={}", target_wasm_path.display()))
            .kill_on_drop(true)
            .stderr(Stdio::piped())
            .output()
            .await?;

        // delete ".o" file
        drop(TmpPath::from(target_path.with_extension("wasm.o")));

        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(color_eyre::Report::msg(message.into_owned()).into());
        }

        let content = tokio::fs::read(&target_wasm_path).await?;
        drop(target_wasm_path);
        return Ok(content);
    }
}
