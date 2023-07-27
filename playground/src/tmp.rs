use std::{
    ffi::{OsStr, OsString},
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
use tracing::info;

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

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct TmpPath(ManuallyDrop<PathBuf>);

impl Deref for TmpPath {
    type Target = Path;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<OsStr> for TmpPath {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl AsRef<Path> for TmpPath {
    #[inline]
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl<T: Into<PathBuf>> From<T> for TmpPath {
    #[inline]
    fn from(value: T) -> Self {
        TmpPath(ManuallyDrop::new(value.into()))
    }
}

impl Drop for TmpPath {
    fn drop(&mut self) {
        let path = unsafe { ManuallyDrop::take(&mut self.0) };
        tokio::task::spawn_blocking(move || tri!(std::fs::remove_file(path)));
    }
}

#[derive(Debug)]
pub struct TmpFile {
    inner: ManuallyDrop<File>,
    path: ManuallyDrop<PathBuf>,
}

impl TmpFile {
    pub async fn new(extension: impl AsRef<OsStr>) -> std::io::Result<Self> {
        match tokio::fs::create_dir(".tmp/").await {
            Err(e) if e.kind() != ErrorKind::AlreadyExists => return Err(e),
            _ => {}
        }

        let mut options = tokio::fs::OpenOptions::new();
        options.write(true);
        options.create_new(true);

        let mut path = OsString::new();
        let inner = loop {
            path.clear();
            path.write_fmt(format_args!("./.tmp/{}.", rand::random::<u64>()))
                .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;
            path.push(extension.as_ref());

            info!(
                "Trying to create {}",
                AsRef::<Path>::as_ref(&path).display()
            );

            match options.open(&path).await {
                Ok(file) => break file,
                Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        };

        return Ok(Self {
            inner: ManuallyDrop::new(inner),
            path: ManuallyDrop::new(PathBuf::from(path)),
        });
    }

    pub async fn drop_handle(self) -> std::io::Result<TmpPath> {
        unsafe {
            let mut this = ManuallyDrop::new(self);
            let path = TmpPath(core::ptr::read(&this.path));

            this.inner.flush().await?;
            ManuallyDrop::drop(&mut this.inner);
            return Ok(path);
        }
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
        let mut file = unsafe { ManuallyDrop::take(&mut self.inner) };
        let path = unsafe { ManuallyDrop::take(&mut self.path) };

        tokio::spawn(async move {
            tri!(file.shutdown().await);
            tri!(tokio::fs::remove_file(path).await);
        });
    }
}
