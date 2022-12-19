use crate::types::{Error, Result};
use async_trait::async_trait;
use std::path::Path;
use walkdir::WalkDir;

#[async_trait]
pub trait FileStorage : Sync + Send {
    async fn upload_folder(&self, path: &Path, remote_path: &Path) -> Result<()> {
        for entry in WalkDir::new(path) {
            let entry = entry
                .map_err::<Error, _>(|err| std::io::Error::from(err).into())?;
            if !entry.file_type().is_file() {
                continue;
            }
            let entry_path = entry.into_path();
            let remote_entry_path = remote_path.join(entry_path.strip_prefix(&path).unwrap());
            // TODO: check if parallel upload is faster
            self.upload_file(entry_path.as_path(), remote_entry_path.as_path()).await?;
        }
        Ok(())
    }

    async fn upload_file(&self, path: &Path, remote_path: &Path) -> Result<()>;

    async fn upload_buffer(&self, bytes: &[u8], remote_path: &Path) -> Result<()>;
}
