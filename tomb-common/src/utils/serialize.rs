use crate::types::{
    blockstore::car::carv2::blockstore::BlockStore,
    config::{error::ConfigError, keys::manager::Manager},
};
use anyhow::Result;
use rand::thread_rng;
use std::{collections::BTreeMap, rc::Rc, vec};
use wnfs::{
    common::{AsyncSerialize, BlockStore as WnfsBlockStore, HashOutput},
    libipld::{serde as ipld_serde, Cid, Ipld},
    private::{
        PrivateDirectory, PrivateForest, PrivateNode, PrivateRef, RsaPrivateKey, TemporalKey, PrivateNodeOnPathHistory,
    },
};

/// Store a given PrivateForest in a given Store
async fn store_forest(forest: &Rc<PrivateForest>, store: &impl WnfsBlockStore) -> Result<Cid> {
    // Create an IPLD from the PrivateForest
    let forest_ipld = forest.async_serialize_ipld(store).await?;
    // Store the PrivateForest's IPLD in the BlockStore
    let ipld_cid = store.put_serializable(&forest_ipld).await?;
    // Return Ok
    Ok(ipld_cid)
}

/// Load a given PrivateForest from a given Store
async fn load_forest(cid: &Cid, store: &impl WnfsBlockStore) -> Result<Rc<PrivateForest>> {
    // Deserialize the IPLD DAG of the PrivateForest
    let forest_ipld: Ipld = store.get_deserializable(cid).await?;
    // Create a PrivateForest from that IPLD DAG
    let forest: Rc<PrivateForest> =
        Rc::new(ipld_serde::from_ipld::<PrivateForest>(forest_ipld).unwrap());
    // Return
    Ok(forest)
}

/// Store a PrivateDirectory
async fn store_dir(
    metadata: &BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
) -> Result<(Cid, TemporalKey)> {
    // Random number generator
    let rng = &mut thread_rng();

    // Store the root of the PrivateDirectory in the PrivateForest, retrieving a PrivateRef to it
    let dir_ref: PrivateRef = root_dir.store(metadata_forest, metadata, rng).await?;

    // Extract the component fields of the PrivateDirectory's PrivateReference
    let PrivateRef {
        saturated_name_hash,
        temporal_key,
        content_cid,
    } = dir_ref;

    // Store it in the Metadata BlockStore
    let ref_cid = metadata
        .put_serializable::<(HashOutput, Cid)>(&(saturated_name_hash, content_cid))
        .await?;

    // Return OK
    Ok((ref_cid, temporal_key))
}

/// Load a PrivateDirectory
async fn load_dir(
    metadata: &BlockStore,
    temporal_key: &TemporalKey,
    private_ref_cid: &Cid,
    metadata_forest: &Rc<PrivateForest>,
) -> Result<Rc<PrivateDirectory>> {
    // Construct the saturated name hash
    let (saturated_name_hash, content_cid): (HashOutput, Cid) = metadata
        .get_deserializable::<(HashOutput, Cid)>(private_ref_cid)
        .await?;

    // Reconstruct the PrivateRef
    let dir_ref: PrivateRef =
        PrivateRef::with_temporal_key(saturated_name_hash, temporal_key.clone(), content_cid);

    // Load the PrivateDirectory from the PrivateForest
    PrivateNode::load(&dir_ref, metadata_forest, metadata)
        .await?
        .as_dir()
}

// async fn store_key_manager() {}

// async fn load_key_manager(
//     metadata: &BlockStore,
//     key_manager_cid: &Cid
// ) {
//     ;
// }

/// Store everything at once!
pub async fn store_all(
    metadata: &BlockStore,
    content: &BlockStore,
    metadata_forest: &mut Rc<PrivateForest>,
    content_forest: &mut Rc<PrivateForest>,
    root_dir: &Rc<PrivateDirectory>,
    key_manager: &Manager,
) -> Result<()> {
    // Construct new map for metadata
    let mut metadata_map = BTreeMap::new();
    // Store PrivateDirectory in the metadata BlockStore, retrieving the new TemporalKey and cid of remaining PrivateRef components
    let (private_ref_cid, temporal_key) = store_dir(metadata, metadata_forest, root_dir).await?;
    // Update the temporal key in the key manager
    key_manager.update_temporal_key(&temporal_key).await?;
    // If we've yet to initialize our originals
    if let Some(metadata_root) = metadata.get_root() && 
        let Ok(Ipld::Map(map)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
        let Some(Ipld::Link(_)) = map.get("original_private_ref") { } else {
            println!("storing original private ref and original key!");
        // Insert private ref and set original key in key manager
        metadata_map.insert("original_private_ref".to_string(), Ipld::Link(private_ref_cid));
        key_manager.set_original_key(&temporal_key).await?;
    }
    // Put the key manager in
    let key_manager_cid = metadata.put_serializable(key_manager).await?;
    // Store the metadata PrivateForest in the metadata BlockStore
    let metadata_forest_cid = store_forest(metadata_forest, metadata).await?;
    // Insert Links
    metadata_map.insert(
        "metadata_forest".to_string(),
        Ipld::Link(metadata_forest_cid),
    );
    metadata_map.insert("current_private_ref".to_string(), Ipld::Link(private_ref_cid));
    metadata_map.insert("key_manager".to_string(), Ipld::Link(key_manager_cid));
    // Put the metadata IPLD Map into the metadata BlockStore
    let metadata_root_cid = metadata.put_serializable(&Ipld::Map(metadata_map)).await?;
    // Set the root of the metadata BlockStore
    metadata.set_root(&metadata_root_cid);
    // Construct new map for content
    let mut content_map = BTreeMap::new();
    // Store the contetn PrivateForest in the content BlockStore
    let content_forest_cid = store_forest(content_forest, content).await?;
    // Insert Links
    content_map.insert("content_forest".to_string(), Ipld::Link(content_forest_cid));
    // Put the ccontent IPLD Map into the content BlockStore
    let content_root_cid = content.put_serializable(&Ipld::Map(content_map)).await?;
    // Set the root of the content BlockStore
    content.set_root(&content_root_cid);
    Ok(())
}

pub async fn load_history(
    wrapping_key: &RsaPrivateKey,
    metadata: &BlockStore,
    content: &BlockStore
) -> Result<PrivateNodeOnPathHistory> {
    let (metadata_forest, _, current_directory, key_manager) = load_all(wrapping_key, metadata, content).await?;

    // Grab the original key
    let original_key = &key_manager.retrieve_original(wrapping_key).await?;
    if let Some(metadata_root) = metadata.get_root() &&
       let Ok(Ipld::Map(metadata_map)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
       let Some(Ipld::Link(original_private_ref_cid)) = metadata_map.get("original_private_ref") {
            //
           let original_directory = load_dir(metadata, original_key, original_private_ref_cid, &metadata_forest).await?;
       
           PrivateNodeOnPathHistory::of(current_directory, original_directory, 1_000_000, &[], true, metadata_forest, metadata).await
    }
    else {
        Err(ConfigError::MissingMetadata("Missing original PrivateRef in Metadata IPLD".to_string()).into())
    }
}


/// Load everything at once!
pub async fn load_all(
    wrapping_key: &RsaPrivateKey,
    metadata: &BlockStore,
    content: &BlockStore
) -> Result<(
    Rc<PrivateForest>,
    Rc<PrivateForest>,
    Rc<PrivateDirectory>,
    Manager,
)> {
    // The metadata root is valid and the content root is valid
    if let Some(metadata_root) = metadata.get_root() &&
       let Some(content_root) = content.get_root() {
        // If we can grab the Metadata IPLD Map and the Content IPLD Map
        if let Ok(Ipld::Map(metadata_map)) = metadata.get_deserializable::<Ipld>(&metadata_root).await &&
           let Ok(Ipld::Map(content_map)) = content.get_deserializable::<Ipld>(&content_root).await
        {
            // If we are able to find all CIDs
            if let Some(Ipld::Link(metadata_forest_cid)) = metadata_map.get("metadata_forest") &&
            let Some(Ipld::Link(current_private_ref_cid)) = metadata_map.get("current_private_ref") &&
            let Some(Ipld::Link(key_manager_cid)) = metadata_map.get("key_manager") &&
            let Some(Ipld::Link(content_forest_cid)) = content_map.get("content_forest")
            {
                // Load in the objects
                let metadata_forest = load_forest(metadata_forest_cid, metadata).await?;
                let content_forest = load_forest(content_forest_cid, content).await?;
                let key_manager = metadata.get_deserializable::<Manager>(key_manager_cid).await?;
                let current_key = &key_manager.retrieve_current(wrapping_key).await?;
                let current_directory = load_dir(metadata, current_key, current_private_ref_cid, &metadata_forest).await?;
                // Return Ok with loaded objectsd
                Ok((metadata_forest, content_forest, current_directory, key_manager))
            }
            else {
                Err(ConfigError::MissingMetadata("One or both BlockStores are missing CIDs".to_string()).into())
            }
        }
        else {
            Err(ConfigError::MissingMetadata("One or both BlockStores are missing IPLDs".to_string()).into())
        }
    }
    else {
        Err(ConfigError::MissingMetadata("One or both BlockStores are missing roots".to_string()).into())
    }
}

#[cfg(test)]
mod test {
    use crate::utils::{serialize::*, tests::*};
    use anyhow::Result;
    use chrono::Utc;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn serial_forest() -> Result<()> {
        let test_name = "serial_metadata_forest";
        // Start er up!
        let (_, _, config, metadata_forest, _, _) = &mut setup(test_name).await?;

        // Store and load
        let metadata_forest_cid = store_forest(metadata_forest, &config.metadata).await?;
        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, &config.metadata).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, &config.metadata)
                .await?
                .len(),
            0
        );

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_dir_object() -> Result<()> {
        let test_name = "serial_dir_object";
        // Start er up!
        let (_, _, config, metadata_forest, _, dir) = &mut setup(test_name).await?;

        let (private_ref_cid, temporal_key) =
            &store_dir(&config.metadata, metadata_forest, dir).await?;
        let metadata_forest_cid = store_forest(metadata_forest, &config.metadata).await?;
        let new_metadata_forest = &load_forest(&metadata_forest_cid, &config.metadata).await?;
        let new_dir = &mut load_dir(
            &config.metadata,
            temporal_key,
            private_ref_cid,
            new_metadata_forest,
        )
        .await?;
        // Assert equality
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_dir_content() -> Result<()> {
        let test_name = "serial_dir_content";
        // Start er up!
        let (_, _, config, original_metadata_forest, original_content_forest, original_dir) =
            &mut setup(test_name).await?;

        // Grab the original file
        let original_file = original_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                original_metadata_forest,
                &config.metadata,
                &mut thread_rng(),
            )
            .await?;

        // Get the content
        let original_content = original_file
            .get_content(original_content_forest, &config.content)
            .await?;

        let (private_ref_cid, temporal_key) =
            &store_dir(&config.metadata, original_metadata_forest, original_dir).await?;
        let metadata_forest_cid = store_forest(original_metadata_forest, &config.metadata).await?;

        let new_metadata_forest = &mut load_forest(&metadata_forest_cid, &config.metadata).await?;
        let new_dir = &mut load_dir(
            &config.metadata,
            temporal_key,
            private_ref_cid,
            new_metadata_forest,
        )
        .await?;
        // Assert equality
        assert_eq!(original_dir, new_dir);

        let file = new_dir
            .open_file_mut(
                &["cats".to_string()],
                true,
                Utc::now(),
                new_metadata_forest,
                &config.metadata,
                &mut thread_rng(),
            )
            .await?;
        // Get the content
        let new_content = file
            .get_content(original_content_forest, &config.content)
            .await?;

        assert_eq!(original_content, new_content);

        // Teardown
        teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn serial_all() -> Result<()> {
        let test_name = "serial_all";
        // Start er up!
        let (_, global, config, metadata_forest, content_forest, dir) =
            &mut setup(test_name).await?;
        let wrapping_key = global.wrapping_key_from_disk()?;
        let key_manager = &Manager::default();
        key_manager.insert(&wrapping_key.get_public_key()).await?;

        let _ = &store_all(
            &config.metadata,
            &config.content,
            metadata_forest,
            content_forest,
            dir,
            key_manager,
        )
        .await?;

        let (new_metadata_forest, new_content_forest, new_dir, _) =
            &mut load_all(&wrapping_key, &config.metadata, &config.content).await?;

        // Assert equality
        assert_eq!(
            new_metadata_forest
                .diff(metadata_forest, &config.metadata)
                .await?
                .len(),
            0
        );
        assert_eq!(
            new_content_forest
                .diff(content_forest, &config.content)
                .await?
                .len(),
            0
        );
        assert_eq!(dir, new_dir);
        // Teardown
        teardown(test_name).await
    }
}
