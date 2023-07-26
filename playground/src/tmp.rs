use std::{
    ffi::OsString,
    fmt::Write,
    io::ErrorKind,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::{
    fs::File,
    io::{AsyncRead, AsyncSeek, AsyncWrite, AsyncWriteExt},
};

#[derive(Debug)]
pub struct TmpFile {
    inner: ManuallyDrop<File>,
    path: ManuallyDrop<PathBuf>,
}

impl TmpFile {
    pub async fn new() -> color_eyre::Result<Self> {
        tokio::fs::create_dir_all(".tmp").await?;

        let mut options = tokio::fs::OpenOptions::new();
        options.create_new(true);

        let mut path = OsString::new();
        let inner = loop {
            path.clear();
            path.write_fmt(format_args!(".tmp/{}", rand::random::<u64>()))?;

            match options.open(&path).await {
                Ok(file) => break file,
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e.into()),
            }
        };

        return Ok(Self {
            inner: ManuallyDrop::new(inner),
            path: ManuallyDrop::new(PathBuf::from(path)),
        });
    }

    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AsyncRead for TmpFile {
    #[inline]
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::<&mut File>::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for TmpFile {
    #[inline]
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        Pin::<&mut File>::new(&mut self.inner).poll_write(cx, buf)
    }

    #[inline]
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::<&mut File>::new(&mut self.inner).poll_flush(cx)
    }

    #[inline]
    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::<&mut File>::new(&mut self.inner).poll_shutdown(cx)
    }
}

impl AsyncSeek for TmpFile {
    #[inline]
    fn start_seek(mut self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        Pin::<&mut File>::new(&mut self.inner).start_seek(position)
    }

    #[inline]
    fn poll_complete(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        Pin::<&mut File>::new(&mut self.inner).poll_complete(cx)
    }
}

impl Deref for TmpFile {
    type Target = File;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for TmpFile {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Drop for TmpFile {
    #[inline]
    fn drop(&mut self) {
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    Ok(x) => x,
                    Err(e) => {
                        tracing::error!("{e}");
                        return;
                    }
                }
            };
        }

        let mut file = unsafe { ManuallyDrop::take(&mut self.inner) };
        let path = unsafe { ManuallyDrop::take(&mut self.path) };

        tokio::spawn(async move {
            tri!(file.shutdown().await);
            tri!(tokio::fs::remove_file(path).await);
        });
    }
}
