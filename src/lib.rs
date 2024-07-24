#![feature(iter_array_chunks)]
#![feature(fs_try_exists)]

use std::time::Instant;

use metadata_db::{AudioFile, MetadataDatabase};
use vector_db::VectorDatabase;

pub mod feature_extractor;
mod file_utils;
pub mod metadata_db;
pub mod vector_db;

pub fn build_db(
    asset_dir: &str,
    progress_callback: impl Fn(f32),
) -> Result<VectorDatabase, String> {
    let start_time = Instant::now();
    let features = feature_extractor::extract_features(
        feature_extractor::RunMode::Parallel,
        asset_dir,
        progress_callback,
    )
    .unwrap();
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to extract features", elapsed);

    let start_time = Instant::now();
    let db = VectorDatabase::build(&features, asset_dir, feature_extractor::NUM_DIMENSIONS)?;
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
    db.list_audio_files(start_offset, num_results)
}
