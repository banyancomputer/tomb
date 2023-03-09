use crate::{
    types::unpack_plan::{ManifestData, UnpackPipelinePlan},
    vacuum::unpack::do_file_pipeline,
};
use anyhow::Result;
use std::path::PathBuf;
use tokio_stream::StreamExt;

/// Given the input directory, the output directory, and the manifest file
/// unpack the input directory into the output directory
pub async fn unpack_pipeline(
    input_dir: PathBuf,
    output_dir: PathBuf,
    manifest_file: PathBuf,
) -> Result<()> {
    // parse manifest file into Vec<CodablePipeline>
    let reader = std::fs::File::open(manifest_file)?;

    // Deserialize the data read as the latest version of manifestdata
    let manifest_data: ManifestData = serde_json::from_reader(reader)?;

    // Check the version is what we want
    if manifest_data.version != env!("CARGO_PKG_VERSION") {
        // Panic if it's not
        panic!("Unsupported manifest version.");
    }

    // Extract the unpacking plans
    let unpack_plans: Vec<UnpackPipelinePlan> = manifest_data.unpack_plans;

    // Iterate over each pipeline
    tokio_stream::iter(unpack_plans)
        .then(|pipeline_to_disk| {
            do_file_pipeline(pipeline_to_disk, input_dir.clone(), output_dir.clone())
        })
        .collect::<Result<Vec<_>>>()
        .await?;

    // If the async block returns, we're Ok.
    Ok(())
}
