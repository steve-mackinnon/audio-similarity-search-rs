#![feature(iter_array_chunks)]
#![feature(fs_try_exists)]

use std::time::Instant;

use feature::Feature;
use metadata_db::{AudioFile, MetadataDatabase};
use vector_db::VectorDatabase;

mod feature;
pub mod feature_extractor;
mod file_utils;
pub mod metadata_db;
pub mod vector_db;

pub fn analyze_and_build_db(
    asset_dir: &str,
    progress_callback: impl Fn(f32),
) -> Result<VectorDatabase, String> {
    let start_time = Instant::now();

    let metadata_db = MetadataDatabase::load_from_disk()?;
    // We cache feature vectors in the SQLite db to avoid re-analyzing samples
    let cached_features = metadata_db.get_all_features()?;
    let mut features: Vec<Feature> = feature_extractor::extract_features(
        feature_extractor::RunMode::Parallel,
        asset_dir,
        &cached_features,
        progress_callback,
    )?;

    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to extract features", elapsed);

    let start_time = Instant::now();
    // Add the newly extracted features to the metadata db
    let dir_id = metadata_db.initialize(asset_dir)?;
    for feature in features.iter_mut() {
        let id = metadata_db.insert_sample_metadata(
            feature.source_file(),
            dir_id,
            feature.feature_vector(),
        )?;
        feature.set_id(id);
    }

    // Combine previously cached features with the new ones
    let db = VectorDatabase::load_from_disk()?;
    db.add_features_to_index(&features, feature_extractor::NUM_DIMENSIONS)?;
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to build database", elapsed);

    Ok(db)
}

pub fn find_similar(source_id: u32, num_results: usize) -> Result<Vec<AudioFile>, String> {
    // Otherwise, load the existing db from disk and query it
    let vec_db = VectorDatabase::load_from_disk()?;
    let ids = vec_db.find_similar(source_id, num_results)?;
    let md_db = MetadataDatabase::load_from_disk()?;
    md_db.get_audio_files_for_ids(&ids)
}

pub fn list_audio_files(start_offset: u32, num_results: u32) -> Result<Vec<AudioFile>, String> {
    let db = MetadataDatabase::load_from_disk()?;
    db.list_audio_files(start_offset, Some(num_results))
}
