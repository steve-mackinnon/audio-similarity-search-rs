use audio_similarity_search::feature_extractor;
use audio_similarity_search::vector_db::VectorDatabase;
use core::panic;
use std::{env, time::Instant};

enum Mode {
    Build,
    Search,
    ListSamples,
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        panic!("Invalid number of args provided");
    }
    let mode = &args[1];
    let run_mode = match mode as &str {
        "build" => Mode::Build,
        "search" => Mode::Search,
        "list-samples" => Mode::ListSamples,
        value => panic!("Invalid mode provided as first argument: {}", value),
    };

    match run_mode {
        Mode::Build => {
            if args.len() != 3 {
                panic!("Invalid args provided for build mode");
            }
            let audio_asset_dir = &args[2];
            build_database(audio_asset_dir);
        }
        Mode::ListSamples => {
            list_samples();
        }
        Mode::Search => {
            if args.len() != 4 {
                panic!("Invalid args provided for search mode. Please pass the source sample ID and num results");
            }
            let source_id = args[2].parse::<u32>().unwrap();
            let num_results = args[3].parse::<usize>().unwrap();
            find_similar(source_id, num_results);
        }
    };
}

fn build_database(asset_dir: &str) {
    let start_time = Instant::now();
    let features =
        feature_extractor::extract_features(feature_extractor::RunMode::Parallel, asset_dir)
            .unwrap();
    feature_extractor::save_to_file(&features).unwrap();
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to extract features", elapsed);

    let start_time = Instant::now();
    let db = VectorDatabase::from_features(&features, feature_extractor::NUM_DIMENSIONS).unwrap();
    if let Ok(results) = db.find_similar(0, 10) {
        for result in results {
            println!("{result}");
        }
    }
    let elapsed = start_time.elapsed();
    println!("Took {:.1?} to build database", elapsed);
}

fn find_similar(source_id: u32, num_results: usize) {
    // Otherwise, load the existing db from disk and query it
    let db = VectorDatabase::load_from_disk().unwrap();
    if let Ok(results) = db.find_similar(source_id, num_results) {
        for result in results {
            println!("{result}");
        }
    }
}

fn list_samples() {
    let features = feature_extractor::from_file().unwrap();
    for (id, path) in features.iter() {
        println!("{}: {}", id, path);
    }
}
