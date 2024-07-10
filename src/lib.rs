#![feature(iter_array_chunks)]
#![feature(fs_try_exists)]

use std::time::Instant;

pub use feature_extractor::FeatureMetadata;
use vector_db::VectorDatabase;

pub mod feature_extractor;
mod file_utils;
pub mod vector_db;

pub fn build_db(asset_dir: &str) -> Result<VectorDatabase, String> {
    let start_time = Instant::now();
    let features =
        feature_extractor::extract_features(feature_extractor::RunMode::Parallel, asset_dir)
            .unwrap();
    feature_extractor::save_to_file(&features, asset_dir.to_string())?;
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to extract features", elapsed);

    let start_time = Instant::now();
    let db =
        VectorDatabase::from_features(&features, feature_extractor::NUM_DIMENSIONS, asset_dir)?;
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to build database", elapsed);

    Ok(db)
}

/// A valid asset_dir can be provided to verify that entries for the requested
/// asset directory were found in the db. If not, an error will be returned.
pub fn load_db_from_disk(asset_dir: Option<&str>) -> Result<VectorDatabase, String> {
    VectorDatabase::load_from_disk(asset_dir)
}

pub fn load_feature_metadata_from_disk() -> Result<FeatureMetadata, String> {
    feature_extractor::from_file(None)
}

/// A valid asset_dir can be provided to verify that audio files for the requested
/// asset directory were found. If not, an error will be returned.
pub fn list_audio_files(asset_dir: Option<&str>) -> Result<Vec<String>, String> {
    // TODO: right now this lists 100 arbitrary files. Ideally the files
    // would be returned in order, and the client could request an arbitrary
    // subset of files.
    Ok(feature_extractor::from_file(asset_dir)?
        .feature_map()
        .values()
        .take(100)
        .map(|v| v.to_string())
        .collect())
}

/// A valid asset_dir can be provided to verify that entries for the requested
/// asset directory were found. If not, an error will be returned.
pub fn find_similar(
    source_id: u32,
    num_results: usize,
    asset_dir: Option<&str>,
) -> Result<Vec<String>, String> {
    // Otherwise, load the existing db from disk and query it
    let db = VectorDatabase::load_from_disk(asset_dir)?;
    db.find_similar(source_id, num_results)
}
