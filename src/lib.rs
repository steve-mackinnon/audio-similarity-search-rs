#![feature(iter_array_chunks)]
#![feature(fs_try_exists)]

use std::time::Instant;

use vector_db::VectorDatabase;

pub mod feature_extractor;
mod file_utils;
pub mod vector_db;

pub fn build_db(asset_dir: &str) -> Result<VectorDatabase, String> {
    let start_time = Instant::now();
    let features =
        feature_extractor::extract_features(feature_extractor::RunMode::Parallel, asset_dir)
            .unwrap();
    feature_extractor::save_to_file(&features)?;
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to extract features", elapsed);

    let start_time = Instant::now();
    let db = VectorDatabase::from_features(&features, feature_extractor::NUM_DIMENSIONS)?;
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to build database", elapsed);

    Ok(db)
}

pub fn load_db_from_disk() -> Result<VectorDatabase, String> {
    VectorDatabase::load_from_disk()
}

pub fn list_audio_files() -> Result<Vec<String>, String> {
    // TODO: right now this lists 100 arbitrary files. Ideally the files
    // would be returned in order, and the client could request an arbitrary
    // subset of files.
    Ok(feature_extractor::from_file()?
        .values()
        .take(100)
        .map(|v| v.to_string())
        .collect())
}
