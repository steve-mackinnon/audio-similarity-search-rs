use audio_similarity_search::{build_db, feature_extractor, find_similar};
use core::panic;
use std::env;

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
                panic!("Invalid args");
            }
            let audio_asset_dir = &args[2];
            let _ = build_db(audio_asset_dir);
        }
        Mode::ListSamples => {
            if args.len() != 2 {
                panic!("Invalid args");
            }
            list_samples();
        }
        Mode::Search => {
            if args.len() != 4 {
                panic!("Invalid args provided for search mode. Please pass the source sample ID and num results");
            }
            let source_id = args[2].parse::<u32>().unwrap();
            let num_results = args[3].parse::<usize>().unwrap();
            if let Ok(results) = find_similar(source_id, num_results, None) {
                for result in results.iter() {
                    println!("{result}");
                }
            }
        }
    };
}

fn list_samples() {
    let features = feature_extractor::from_file(None).unwrap();
    for (id, path) in features.iter() {
        println!("{}: {}", id, path);
    }
}
