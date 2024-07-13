use std::num::NonZeroUsize;

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::fs;

use crate::feature_extractor::Feature;
use crate::file_utils;
use crate::metadata_db::MetadataDatabase;
use arroy::distances::Angular;
use arroy::{Database as ArroyDatabase, Reader, Writer};

/// That's the 200MiB size limit we allow LMDB to grow.
const TWENTY_HUNDRED_MIB: usize = 2 * 1024 * 1024 * 1024;

pub struct VectorDatabase {
    db: ArroyDatabase<Angular>,
}

impl VectorDatabase {
    pub fn load_from_disk() -> Result<VectorDatabase, String> {
        let dir = file_utils::data_directory()?;
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .map_size(TWENTY_HUNDRED_MIB)
                .open(&dir)
        }
        .map_err(|_| format!("Failed to open database from {}", dir.to_string_lossy()))?;

        let mut write_txn = env.write_txn().map_err(|e| e.to_string())?;
        let db: ArroyDatabase<Angular> = env
            .create_database(&mut write_txn, None)
            .map_err(|e| e.to_string())?;

        Ok(VectorDatabase { db })
    }

    /// Builds the database and saves it on disk.
    pub fn build(
        features: &[Feature],
        analysis_root_dir: &str,
        dimensions: usize,
    ) -> Result<VectorDatabase, String> {
        // First, create our metadata db which is used to associate an id with a path
        // to the audio file. Since arroy only allows insertion of an id and a vector,
        // we keep the file path and any other necessary metadata in a separate sqlite db.
        let metadata_db = MetadataDatabase::load_from_disk()?;
        let root_dir_id = metadata_db.initialize(analysis_root_dir)?;

        // TODO: right now we remove an existing db if we find one. Can we append to an existing db
        // if we aren't fully rebuilding the index? If so, we should split this out into a separate
        // clean/full rebuild function.
        let db_path = file_utils::vector_db_path()?;
        if let Ok(true) = fs::try_exists(&db_path) {
            println!("Removing existing database...");
            fs::remove_file(db_path).map_err(|e| format!("Failed to remove existing db: {}", e))?;
        }
        let lock_path = file_utils::db_lock_path()?;
        if let Ok(true) = fs::try_exists(&lock_path) {
            println!("Removing existing lock file...");
            fs::remove_file(lock_path).map_err(|e| format!("Failed to remove lock file: {}", e))?;
        }

        let data_local_dir = file_utils::data_directory()?;
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .map_size(TWENTY_HUNDRED_MIB)
                .open(data_local_dir)
        }
        .map_err(|e| e.to_string())?;
        println!(
            "Vector database intialized at {}",
            env.path().to_string_lossy()
        );

        let mut write_txn = env.write_txn().map_err(|e| e.to_string())?;
        let db: ArroyDatabase<Angular> = env
            .create_database(&mut write_txn, None)
            .map_err(|e| e.to_string())?;

        let index = 0;
        let writer = Writer::<Angular>::new(db, index, dimensions);
        // Add features
        for feature in features.iter() {
            // First, write to the sqlite db to store metadata an obtain an id
            let id = metadata_db.insert_sample_metadata(feature.source_file(), root_dir_id)?;
            // Write to the annoy vector db using the id from the sqlite table
            writer
                .add_item(&mut write_txn, id as u32, feature.feature_vector())
                .map_err(|e| e.to_string())?;
        }
        let mut rng = StdRng::from_entropy();
        let num_trees = None;
        // Build index
        writer
            .build(&mut write_txn, &mut rng, num_trees)
            .map_err(|e| e.to_string())?;

        // Commit the built index to the db
        write_txn.commit().map_err(|e| e.to_string())?;

        Ok(VectorDatabase { db })
    }

    /// Returns a vector of file ids to the top k similar results
    pub fn find_similar(&self, id: u32, num_results: usize) -> Result<Vec<u32>, String> {
        let data_local_dir = file_utils::data_directory()?;
        let env = unsafe {
            heed::EnvOpenOptions::new()
                .map_size(TWENTY_HUNDRED_MIB)
                .open(data_local_dir)
        }
        .map_err(|e| e.to_string())?;

        let rtxn = env.read_txn().map_err(|e| e.to_string())?;
        let index = 0;
        let reader = Reader::<Angular>::open(&rtxn, index, self.db).map_err(|e| e.to_string())?;

        // You can increase the quality of the results by forcing arroy to search into more nodes.
        // This multiplier is arbitrary but basically the higher, the better the results, the slower the query.
        let search_k = NonZeroUsize::new(num_results * reader.n_trees() * 15);

        // Similar searching can be achieved by requesting the nearest neighbors of a given item.
        let search_results = reader
            .nns_by_item(&rtxn, id, num_results, search_k, None)
            .map_err(|e| e.to_string())?
            .unwrap()
            .iter()
            .map(|result| result.0)
            .collect();
        Ok(search_results)
    }
}
