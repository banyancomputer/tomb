use std::path::Path;

use crate::{utils::all_bfs_paths, NativeError};
use banyanfs::{
    filesystem::{Drive, OperationError},
    stores::DataStore,
};
use tokio::{
    fs::{create_dir, OpenOptions},
    io::AsyncWriteExt,
};
use tokio_util::compat::{
    FuturesAsyncWriteCompatExt, TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt,
};
use tracing::info;

pub async fn restore(
    drive: &mut Drive,
    store: &mut impl DataStore,
    output: &Path,
) -> Result<(), NativeError> {
    if !output.exists() {
        create_dir(output).await?;
    }
    let mut file_opts = OpenOptions::new();
    file_opts.write(true);
    file_opts.create(true);
    file_opts.truncate(true);

    let paths = all_bfs_paths(drive).await?;
    println!("paths: {:?}", paths);

    let root = drive.root().await?;
    for path in all_bfs_paths(drive).await? {
        // Disk location
        let canon = output.join(&path);
        info!("canon: {}", canon.display());
        // Path on FS
        let bfs_path: Vec<&str> = path
            .components()
            .filter_map(|v| v.as_os_str().to_str())
            .collect();
        // Attempt to read from the banyan filesystem
        match root.read(store, &bfs_path).await {
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
                info!("about to make dir!");
                if !canon.exists() {
                    create_dir(&canon).await?;
                }
            }
            Err(err) => {
                return Err(err.into());
            }
        }
    }

    Ok(())
}
