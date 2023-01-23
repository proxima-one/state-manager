use super::interface::FileStorage;
use crate::types::Result;
use async_trait::async_trait;
use s3::Bucket;
use std::path::Path;

pub struct S3FileStorage {
    bucket: Bucket,
}

impl S3FileStorage {
    pub fn new(s3_bucket: Bucket) -> Self {
        Self { bucket: s3_bucket }
    }
}

#[async_trait]
impl FileStorage for S3FileStorage {
    async fn upload_file(&self, path: &Path, remote_path: &Path) -> Result<()> {
        let mut file = tokio::fs::File::open(path).await?;
        self.bucket
            .put_object_stream(&mut file, remote_path.to_string_lossy())
            .await?;
        Ok(())
    }

    async fn upload_buffer(&self, bytes: &[u8], remote_path: &Path) -> Result<()> {
        self.bucket
            .put_object(remote_path.to_string_lossy(), bytes)
            .await?;
        Ok(())
    }
}
