use audio_similarity_search::{analyze_and_build_db, metadata_db, vector_db::VectorDatabase};
use clap::{Parser, Subcommand};
use metadata_db::MetadataDatabase;

#[derive(Parser, Debug)]
#[command(
    name = "audio-similarity-search",
    about = "A CLI for running similarity search across audio files. First, run analyze to analyze a directory of audio files. Then list-samples, can be used to list the analyzed samples and their IDs. search can be used to find similar samples given a sample ID."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run analysis on the provided directory. Builds a vector database that can be queried to find similar samples using the search command.
    Analyze {
        #[arg(value_name = "SOURCE_DIR")]
        source_dir: String,
    },
    /// Run similarity search for a given sample
    Search {
        /// The source sample ID
        #[arg(value_name = "SAMPLE_ID")]
        id: u32,
        /// How many results to return
        #[arg(value_name = "NUM_RESULTS")]
        num_results: usize,
    },
    /// Lists all analyzed sample paths and their IDs
    List {
        /// OPTIONAL: The maximum number of samples to return
        #[arg(value_name = "LIMIT")]
        limit: Option<u32>,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Analyze { source_dir } => {
            let _ = analyze_and_build_db(&source_dir, |_| {}).unwrap();
        }
        Commands::Search { id, num_results } => {
            let db = VectorDatabase::load_from_disk().unwrap();
            if let Ok(results) = db.find_similar(*id, *num_results) {
                for result in results.iter() {
                    println!("{result}");
                }
            }
        }
        Commands::List { limit } => {
            list_samples(*limit);
        }
    }
}

fn list_samples(limit: Option<u32>) {
    let db = MetadataDatabase::load_from_disk().unwrap();
    let files = db.list_audio_files(0, limit).unwrap();
    for file in files.iter() {
        println!("{}", file.path());
    }
}
