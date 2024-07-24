use rodio::{source::Source, Decoder};
use rubato::Resampler;
use rubato::{SincFixedIn, SincInterpolationParameters, SincInterpolationType};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::mpsc;
use threadpool::ThreadPool;
use walkdir::WalkDir;

fn get_audio_files(root_dir: &str) -> Vec<String> {
    let path = PathBuf::from(root_dir);

    let supported_extensions = ["wav", "mp3"];
    WalkDir::new(path)
        .into_iter()
        .filter_map(|d| d.ok())
        .map(|d| d.path().to_owned())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| supported_extensions.contains(&ext))
                .unwrap_or(false)
        })
        .map(|path| path.into_os_string().into_string().unwrap())
        .collect()
}

pub enum RunMode {
    SingleThreaded,
    Parallel,
}

pub struct Feature {
    feature_vector: Vec<f32>,
    source_file: String,
}

impl Feature {
    pub fn new(feature_vector: Vec<f32>, source_file: String) -> Self {
        Self {
            feature_vector,
            source_file,
        }
    }

    pub fn feature_vector(&self) -> &[f32] {
        &self.feature_vector
    }

    pub fn source_file(&self) -> &str {
        &self.source_file
    }
}

pub const NUM_DIMENSIONS: usize = 13;

pub fn extract_features(
    run_mode: RunMode,
    asset_dir: &str,
    progress_callback: impl Fn(f32),
) -> Result<Vec<Feature>, String> {
    let files = get_audio_files(asset_dir);
    let num_files = files.len();
    if num_files == 0 {
        return Err(format!("No files found in {asset_dir}"));
    }

    let mut features: Vec<Feature> = Vec::with_capacity(files.len());

    match run_mode {
        RunMode::SingleThreaded => {
            for file in files.iter() {
                if let Ok(mfcc) = decode_and_calculate_mfcc(file, 22050) {
                    features.push(Feature::new(mfcc, file.to_string()));
                }
            }
        }
        RunMode::Parallel => {
            let num_threads = num_cpus::get();
            println!("Running with {num_threads} threads");
            let thread_pool = ThreadPool::new(num_threads);

            let (sender, receiver) = mpsc::channel::<Feature>();

            for file in files.iter() {
                let f = file.to_string();
                let sender = sender.clone();
                thread_pool.execute(move || {
                    if let Ok(mfcc) = decode_and_calculate_mfcc(&f, 22050) {
                        sender.send(Feature::new(mfcc, f)).unwrap();
                    } else {
                        println!("Failed to extract features for {f}");
                    }
                });
            }

            let mut progress = 0.0;
            let progress_increment = 1.0 / files.len() as f32;
            while thread_pool.active_count() > 0 || thread_pool.queued_count() > 0 {
                if let Ok(feature) = receiver.try_recv() {
                    features.push(feature);
                    progress += progress_increment;
                    progress_callback(progress);
                }
            }
        }
    }
    Ok(features)
}

fn decode_and_calculate_mfcc(path: &str, output_sample_rate: u32) -> Result<Vec<f32>, String> {
    let mut decoded = decode_and_resample_file(path, output_sample_rate).unwrap();
    let mfcc = calculate_mfcc(&mut decoded, 22050);
    match mfcc {
        Ok(mfcc) => {
            println!("MFCC: {:?}", mfcc);
            Ok(mfcc)
        }
        Err(e) => {
            println!("{}", e);
            Err(e)
        }
    }
}

fn decode_and_resample_file(path: &str, output_sample_rate: u32) -> Result<Vec<f32>, String> {
    let file = BufReader::new(File::open(path).map_err(|e| e.to_string())?);
    let decoder = Decoder::new(file).map_err(|e| e.to_string())?;
    let num_channels = decoder.channels();
    let sample_rate = decoder.sample_rate();

    let mut samples: Vec<f32>;
    if num_channels == 1 {
        samples = decoder.convert_samples::<f32>().collect();
    } else if num_channels == 2 {
        // Sum to mono
        samples = decoder
            .convert_samples::<f32>()
            .array_chunks::<2>()
            .map(|frame: [f32; 2]| (frame[0] + frame[1]) * 0.5)
            .collect();
    } else {
        return Err("Unsupported channel count".to_string());
    }

    if sample_rate != output_sample_rate {
        samples = resample_buffer(&samples, sample_rate as f64, output_sample_rate as f64);
    }
    // TODO: write to file to verify quality
    Ok(samples)
}

fn resample_buffer(buffer: &Vec<f32>, source_sr: f64, dest_sr: f64) -> Vec<f32> {
    let max_resample_ratio_relative: f64 = 10.0;
    let chunk_size = 2048;
    let num_channels = 1;

    let params = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: SincInterpolationType::Linear,
        oversampling_factor: 160,
        window: rubato::WindowFunction::BlackmanHarris2,
    };

    // Create the resampler
    let mut resampler = SincFixedIn::<f32>::new(
        dest_sr / source_sr,
        max_resample_ratio_relative,
        params,
        chunk_size,
        num_channels,
    )
    .unwrap();

    let mut input: Vec<&[f32]> = vec![buffer];
    let mut input_offset = 0;
    let mut resampled_buffer: Vec<f32> = Vec::with_capacity(buffer.len());
    let mut output_buffer: Vec<Vec<f32>> = vec![vec![0.0; 2048]];

    while let Ok((input_frames, output_frames)) =
        resampler.process_into_buffer(&input, &mut output_buffer, None)
    {
        let output = output_buffer.first().unwrap();
        resampled_buffer.extend_from_slice(&output[0..output_frames]);
        input_offset += input_frames;
        let next_input = &buffer[input_offset..];
        input[0] = next_input;
    }
    resampled_buffer
}

fn calculate_mfcc(buffer: &mut Vec<f32>, sample_rate: u32) -> Result<Vec<f32>, String> {
    let fft_size = 2048;
    let num_coefficients = NUM_DIMENSIONS;
    let num_filters = 40;

    // Pad with zeros if the buffer isn't large enough to hold a full fft block
    let num_blocks = (buffer.len() as f32 / fft_size as f32).floor() as usize;
    if num_blocks == 0 {
        buffer.resize(fft_size, 0.0);
    }

    let mut fft = aubio_rs::FFT::new(fft_size).map_err(|e| e.to_string())?;
    let mut fft_scratch: Vec<f32> = vec![0.0; fft_size];

    let mut mfcc = aubio_rs::MFCC::new(fft_size, num_filters, num_coefficients, sample_rate)
        .map_err(|e| e.to_string())?;
    let mut mean_mfcc: Vec<f32> = vec![0.0; num_coefficients];
    let mut mfcc_scratch: Vec<f32> = vec![0.0; num_coefficients];

    for block_index in 0..num_blocks {
        let start = block_index * fft_size;
        let buf = &buffer[start..];

        fft.do_(buf, &mut fft_scratch).map_err(|_| "FFT failed")?;
        mfcc.do_(&fft_scratch, &mut mfcc_scratch)
            .map_err(|_| "MFCC failed")?;

        for (new, mean) in mfcc_scratch.iter().zip(mean_mfcc.iter_mut()) {
            *mean += new;
        }
    }
    // Calculate mean by dividing by the number of blocks
    for e in &mut mean_mfcc {
        *e /= num_blocks as f32;
    }
    Ok(mean_mfcc)
}
