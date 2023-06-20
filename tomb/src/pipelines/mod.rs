/// This module contains the add pipeline function, which is the main entry point for inserting into existing WNFS filesystems.
pub mod add;
/// This module contains configuration functions for the cli
pub mod configure;
mod error;
/// This module contains the pack pipeline function, which is the main entry point for packing new data.
pub mod pack;
/// This module contains the pull pipeline function, which downloads packed content from disk to a remote server.
pub mod pull;
/// This module contains the push pipeline function, which uploads packed content from disk to a remote server.
pub mod push;
/// This module contains the add pipeline function, which is the main entry point for removing from existing WNFS filesystems.
pub mod remove;
/// This module contains the unpack pipeline function, which is the main entry point for extracting previously packed data.
pub mod unpack;

#[cfg(test)]
mod test {
    use super::add;
    use crate::{
        pipelines::{configure, pack, pull, push, remove, unpack},
        utils::{
            spider::path_to_segments,
            tests::{test_setup, test_setup_structured, test_teardown},
            wnfsio::decompress_bytes,
        },
    };
    use anyhow::Result;
    use dir_assert::assert_paths;
    use fake_file::{Strategy, Structure};
    use serial_test::serial;
    use std::{
        fs::{self, create_dir_all, metadata, File},
        io::Write,
        path::PathBuf,
    };

    #[tokio::test]
    #[serial]
    async fn pipeline_init() -> Result<()> {
        let test_name = "pipeline_init";
        // Create the setup conditions
        let input_dir = &test_setup(test_name).await?;
        // Deinitialize for user
        configure::deinit(input_dir)?;
        // Assert that packing fails
        assert!(pack::pipeline(input_dir, true).await.is_err());
        // Initialize for this user
        configure::init(input_dir)?;
        // Assert that packing succeeds
        assert!(pack::pipeline(input_dir, true).await.is_ok());
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn pipeline_configure_remote() -> Result<()> {
        let test_name = "pipeline_configure_remote";
        // Create the setup conditions
        let input_dir = test_setup(test_name).await?;
        // Configure the remote endpoint
        configure::remote("http://127.0.0.1", 5001)?;

        // Teardown
        test_teardown(test_name).await
    }

    /*
    #[tokio::test]
    async fn pipeline_pack_unpack_local() -> Result<()> {
        let test_name = "pipeline_pack_unpack_local";
        // Create the setup conditions
        let input_dir= &test_setup(test_name).await?;
        // Pack locally
        pack::pipeline(input_dir, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &input_dir.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn pipeline_pack_push() -> Result<()> {
        let test_name = "pipeline_pack_pull_unpack";
        // Create the setup conditions
        let input_dir = &test_setup(test_name).await?;
        // Configure the remote endpoint
        configure::remote("http://127.0.0.1", 5001)?;
        // Pack locally
        pack::pipeline(input_dir, true).await?;
        // Push
        push::pipeline(input_dir).await?;
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn pipeline_pack_push_pull() -> Result<()> {
        let test_name = "pipeline_pack_push_pull";
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init()?;
        // Configure the remote endpoint
        configure::remote(&input_dir, "http://127.0.0.1", 5001)?;
        // Pack locally
        pack::pipeline(&input_dir, &output_dir, 262144, true).await?;
        // Send data to remote endpoint
        push::pipeline(&output_dir).await?;

        // Compute size of original content
        let d1 = metadata(&output_dir.join("content.car"))?.len();
        // Oh no! File corruption, we lost all our data!
        fs::remove_file(output_dir.join("content.car"))?;
        // Now its time to reconstruct all our data
        pull::pipeline(&output_dir).await?;
        // Compute size of reconstructed content
        let d2 = metadata(&output_dir.join("content.car"))?.len();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(d1, d2);
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn pipeline_add_local() -> Result<()> {
        let test_name = "pipeline_add_local";
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Run the pack pipeline
        pack::pipeline(input_dir, &output_dir, 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // This is still in the input dir. Technically we could just
        let input_file = &input_dir.join("hello.txt");
        // Content to be written to the file
        let file_content = String::from("This is just example text.")
            .as_bytes()
            .to_vec();
        // Create and write to the file
        File::create(input_file)?.write_all(&file_content)?;
        // Add the input file to the WNFS
        add::pipeline(input_file, tomb_path, input_file).await?;
        // Now that the pipeline has run, grab all metadata
        let (_, manifest, metadata_forest, content_forest, dir) =
            &mut all_from_disk(tomb_path).await?;
        // Grab the file at this path
        let file = dir
            .get_node(
                &path_to_segments(&input_file)?,
                true,
                metadata_forest,
                &manifest.metadata,
            )
            .await?
            .unwrap()
            .as_file()?;
        // Get the content of the PrivateFile and decompress it
        let mut loaded_file_content: Vec<u8> = Vec::new();
        decompress_bytes(
            file.get_content(content_forest, &manifest.content)
                .await?
                .as_slice(),
            &mut loaded_file_content,
        )?;
        // Assert that the data matches the original data
        assert_eq!(file_content, loaded_file_content);
        // Teardown
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn pipeline_remove_local() -> Result<()> {
        let test_name = "pipeline_remove_local";
        // Create the setup conditions
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        configure::init(input_dir)?;
        // Run the pack pipeline
        pack::pipeline(input_dir, &output_dir, 262144, true).await?;
        // Grab metadata
        let tomb_path = &output_dir.join(".tomb");
        // Write out a reference to where we expect to find this file
        let wnfs_path = &PathBuf::from("").join("0").join("0");
        let wnfs_segments = &path_to_segments(wnfs_path)?;
        // Load metadata
        let (_, manifest, metadata_forest, dir) = &mut hot_from_disk(tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, metadata_forest, &manifest.metadata)
            .await?;
        // Assert the node exists presently
        assert!(result.is_some());
        // Remove the PrivateFile at this Path
        remove::pipeline(tomb_path, wnfs_path).await?;
        // Reload metadata
        let (_, manifest, metadata_forest, dir) = &mut hot_from_disk(tomb_path).await?;
        let result = dir
            .get_node(wnfs_segments, true, metadata_forest, &manifest.metadata)
            .await?;
        // Assert the node no longer exists
        assert!(result.is_none());
        // Teardown
        test_teardown(test_name).await
    }

    // Helper function for structure tests
    async fn assert_pack_unpack_local(test_name: &str) -> Result<()> {
        // Grab directories
        let root_path = PathBuf::from("test").join(test_name);
        let (input_dir, output_dir) = &(root_path.join("input"), root_path.join("output"));
        // Initialize
        configure::init(input_dir)?;
        // Pack locally
        pack::pipeline(input_dir, output_dir, 262144, true).await?;
        // Create a new dir to unpack in
        let unpacked_dir = &output_dir.parent().unwrap().join("unpacked");
        create_dir_all(unpacked_dir)?;
        // Run the unpacking pipeline
        unpack::pipeline(output_dir, unpacked_dir).await?;
        // Assert the pre-packed and unpacked directories are identical
        assert_paths(input_dir, unpacked_dir).unwrap();

        Ok(())
    }

    const STRUCTURE_INPUT_SIZE: usize = 1024;

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_simple() -> Result<()> {
        let test_name = "pipeline_structure_simple";
        let structure = Structure::new(4, 4, STRUCTURE_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_deep() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(2, 8, STRUCTURE_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn pipeline_structure_wide() -> Result<()> {
        let test_name = "pipeline_structure_deep";
        let structure = Structure::new(16, 1, STRUCTURE_INPUT_SIZE, Strategy::Simple);
        test_setup_structured(test_name, structure).await?;
        assert_pack_unpack_local(test_name).await?;
        test_teardown(test_name).await
    }
     */
}
