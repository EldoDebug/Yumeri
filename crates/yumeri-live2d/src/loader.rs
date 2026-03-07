use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum LoaderError {
    #[error("failed to read file: {path}")]
    ReadFailed {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("utf-8 decode failed: {path}")]
    Utf8Failed {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
}

pub trait AssetLoader {
    fn load_bytes(&self, path: &Path) -> Result<Vec<u8>, LoaderError>;

    fn load_string(&self, path: &Path) -> Result<String, LoaderError> {
        let bytes = self.load_bytes(path)?;
        String::from_utf8(bytes).map_err(|source| LoaderError::Utf8Failed {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct StdFsLoader;

impl AssetLoader for StdFsLoader {
    fn load_bytes(&self, path: &Path) -> Result<Vec<u8>, LoaderError> {
        std::fs::read(path).map_err(|source| LoaderError::ReadFailed {
            path: path.to_path_buf(),
            source,
        })
    }
}
