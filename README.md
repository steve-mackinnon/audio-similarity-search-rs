# Audio Similarity Search in Rust

This repo contains a Rust CLI that can be used to perform similarity search across audio files stored locally.

## Building

To build, run:

- `cargo build` to build in debug
- `cargo build --release` to build in release

This will create an executable called `similarity-search` in the `target/release` or `target/debug` directory.

## Running

The CLI supports three modes: building, listing samples, and searching.

### Build

To build a new database for your local audio samples, run:

`./similarity-search build \Path\to\my_audio_samples`

This will recursively iterate over my_audio_samples and all subdirectories to build the similarity search database.

### List

To see a list of samples in the database, and their corresponding IDs, run:

`./similarity-search list-samples`

### Search

To query the database to find similar samples, run:

`./similarity-search find sample_id num_results` where sample_id is an integer representing corresponding to input sample you'd like to find similar sounds for.

## Details

### Feature extraction

The feature extraction phase walks over all of the wav and mp3 files found in the asset directory passed to the CLI during `build`. Each audio file is decoded, downsampled to 22050 Hz, and summed to mono. The resulting audio buffer is then chunked into blocks of 2048 samples, which are passed to [aubio](https://github.com/katyo/aubio-rs) to perform an FFT, then an MFCC to distill the buffer down to a 13 dimensional MFCC vector. For each file, the MFCCs from each block are then averaged, resulting in a single 13-element feature vector. This feature extraction process is highly parallelized. It uses a thread pool to fan distribute the feature extraction for each file across all physical cores on the machine.

After feature extraction completes, a hash map of {FileId, FilePath} is then saved to disk. When querying the database, we need to load this map into memory in order to associate database IDs with audio file paths. The [serde-rs](https://github.com/serde-rs/json) JSON library is used to serialize and deserialize the map.

### Database creation and querying

The [arroy](https://docs.rs/arroy/latest/arroy/) database is used to store the feature vectors and perform similarity search. This project is a Rust port of the [annoy](https://github.com/spotify/annoy) C++/Python library from Spotify, which is used for fast approximate nearest neighbor search. Arroy differs slightly in that it is backed by [LMDB](http://www.lmdb.tech/doc/), a high performance, memory mapped database. arroy/LMDB are taking care of all of the details for index creation and ANN search.
