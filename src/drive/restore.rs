use std::path::Path;

use super::local::LocalBanyanFS;
use crate::NativeError;
use banyanfs::filesystem::OperationError;
use tokio::{
    fs::{create_dir, OpenOptions},
    io::AsyncWriteExt,
};
use tokio_util::compat::{
    FuturesAsyncWriteCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};
use tracing::info;

impl LocalBanyanFS {
    pub async fn restore(&self, output: &Path) -> Result<(), NativeError> {
        if !output.exists() {
            create_dir(output).await?;
        }
        let mut file_opts = OpenOptions::new();
        file_opts.write(true);
        file_opts.create(true);
        file_opts.truncate(true);

        let paths = self.all_bfs_paths().await?;
        println!("paths: {:?}", paths);

        let root = self.drive.root().await?;
        for path in self.all_bfs_paths().await? {
            // Disk location
            let canon = output.join(&path);
            info!("canon: {}", canon.display());
            // Path on FS
            let bfs_path: Vec<&str> = path
                .components()
                .filter_map(|v| v.as_os_str().to_str())
                .collect();
            // Attempt to read from the banyan filesystem
            match root.read(&self.store, &bfs_path).await {
                // File
                Ok(data) => {
                    // Write file data!
                    info!("about to write!");
                    file_opts
                        .open(&canon)
                        .await?
                        .compat()
                        .compat_write()
                        .write(&data)
                        .await?;
                }
                // Directory
                Err(OperationError::NotReadable) => {
                    create_dir(&canon).await?;
                }
                Err(err) => {
                    return Err(err.into());
                }
            }
        }

        Ok(())
    }

    /*
    #[async_recursion(?Send)]
    pub async fn restore(
        &mut self,
        prefix: &Path,
        handle: &DirectoryHandle,
    ) -> Result<(), NativeError> {
        for entry in handle.ls(&[]).await? {
            let name = entry.name().to_string();
            let new_prefix = prefix.join(&name);
            let bfs: Vec<&str> = new_prefix
                .components()
                .filter_map(|v| v.as_os_str().to_str())
                .collect();

            match entry.kind() {
                NodeKind::File => {
                    let file_data = handle.read(&self.store, &bfs).await?;
                }
                NodeKind::Directory => {
                    let new_handle = handle.cd(&[&name]).await?;
                    self.restore(&new_prefix, &new_handle).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }
    */
}
