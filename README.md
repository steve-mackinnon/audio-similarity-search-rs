# Audio Similarity Search in Rust

This repo contains a binary CLI and static library for running similarity search across audio files.

## Building

To build the CLI, run:

- `cargo build` to build in debug
- `cargo build --release` to build in release

This will create an executable called `similarity-search` in the `target/release` or `target/debug` directory.

## Running the CLI

The CLI includes help documentation - just run `./audio-similarity-search --help`.

Commands:

- `analyze`: run analysis on the provided directory. Builds a vector database that can be queried to find similar samples using the search command
- `search`: run similarity search for a given sample
- `list`: lists all analyzed sample paths and their IDs. Optional accepts a LIMIT uint parameter to limit the number or result returned.

## Implementation Details

### Feature extraction

The feature extraction phase walks over all of the wav and mp3 files found in the asset directory passed to the CLI during `build`. Each audio file is decoded, downsampled to 22050 Hz, and summed to mono. The resulting audio buffer is then chunked into blocks of 2048 samples, which are passed to [aubio](https://github.com/katyo/aubio-rs) to perform an FFT, then an MFCC to distill the buffer down to a 13 dimensional MFCC vector. For each file, the MFCCs from each block are then averaged, resulting in a single 13-element feature vector. This feature extraction process is highly parallelized. It uses a thread pool to fan distribute the feature extraction for each file across all physical cores on the machine.

_Note:_ this isn't perfect! Temporal infomation is lost when the MFCCs are averaged, which affects the quality of the similarity search results. It's on my todo list to revisit this.

### Database creation and querying

The [arroy](https://docs.rs/arroy/latest/arroy/) database is used to store the feature vectors and perform similarity search. This project is a Rust port of the [annoy](https://github.com/spotify/annoy) C++/Python library from Spotify, which is used for fast approximate nearest neighbor search. Arroy differs slightly in that it is backed by [LMDB](http://www.lmdb.tech/doc/), a high performance, memory mapped database. arroy/LMDB are taking care of all of the details for index creation and ANN search.

Since arroy only stores IDs and vectors, a SQLite database is used to associate file IDs with their paths and feature vectors. This metadata database is used to hydrate similarity search results to include file paths. Arroy has an [open issue](https://github.com/meilisearch/arroy/issues/67) where appending new vectors does not work. To allow clients to append to the existing arroy db efficiently, we re-insert the cached vectors from the metadata db into arroy when analyzing a new directory of audio files.
