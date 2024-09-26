// image_processing.rs
// use crate::app::App;
use crate::utils::{Logger, measure_time, get_memory_usage};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageError};
use std::sync::Arc;
use parking_lot::Mutex;
use crate::app::ConversionProgress;
use std::time::Instant;
use crate::app::ImageDetail;
use std::sync::mpsc::Sender;
use crate::app::ConversionUpdate;

pub fn convert_images(
    input_files: Vec<PathBuf>,
    output_directory: PathBuf,
    resize_enabled: bool,
    width: u32,
    height: u32,
    quality_enabled: bool,
    compression_quality: f32,
    rename_enabled: bool,
    output_filename: String,
    progress: Arc<Mutex<ConversionProgress>>,
    log_messages: Arc<Mutex<Vec<String>>>,
    original_sizes: Arc<Mutex<Vec<u64>>>,
    compressed_sizes: Arc<Mutex<Vec<u64>>>,
    // image_details: Arc<Mutex<Vec<ImageDetail>>>,
    image_details: Arc<Mutex<Vec<ImageDetail>>>,
    sender: Sender<ConversionUpdate>,
) {   let logger = Logger::new(log_messages.clone());
    logger.log("Starting convert_images function".to_string());
    logger.log(get_memory_usage());

    if input_files.is_empty() {
        logger.log("No input files selected".to_string());
        return;
    }

    let total_files = input_files.len();
    logger.log(format!("Total files to process: {}", total_files));

    {
        let mut progress = progress.lock();
        progress.total = total_files;
        progress.completed = 0;
        progress.status = "Starting conversion...".to_string();
    }

    logger.log("Creating thread pool".to_string());
    let _pool = ThreadPoolBuilder::new().build().unwrap();
    let start_time = Instant::now();

    logger.log("Starting parallel iteration over input files".to_string());
    let currently_processing = Arc::new(Mutex::new(None));
    input_files.par_iter().enumerate().for_each(|(index, input_path)| {
        *currently_processing.lock() = Some(index);
        logger.log(format!("Processing file: {}", input_path.display()));
        let (img_result, load_duration) = measure_time(|| load_image(input_path));
        logger.log(format!("Loading image {} took {:?}", input_path.display(), load_duration));

        if let Ok(img) = img_result {
            logger.log("Image loaded successfully".to_string());

            let img = if resize_enabled {
                logger.log("Resizing image".to_string());
                let (resized_img, resize_duration) = measure_time(|| resize_image(img, width, height));
                logger.log(format!("Resizing image took {:?}", resize_duration));
                resized_img
            } else {
                img
            };

            let quality = if quality_enabled { compression_quality } else { 80.0 };
            logger.log(format!("Using quality: {}", quality));

            logger.log("Encoding to WebP".to_string());
            let (webp_result, encode_duration) = measure_time(|| encode_to_webp(&img, quality));
            logger.log(format!("Encoding to WebP took {:?}", encode_duration));

            if let Ok(webp_data) = webp_result {
                logger.log("WebP encoding successful".to_string());

                let output_file_name = if rename_enabled && !output_filename.is_empty() {
                    format!("{}.webp", output_filename)
                } else {
                    input_path.file_stem().unwrap_or_default().to_string_lossy().to_string() + ".webp"
                };
                let output_path = output_directory.join(output_file_name);

                logger.log(format!("Saving WebP file to: {}", output_path.display()));
                let (save_result, save_duration) = measure_time(|| save_webp(&webp_data, &output_path));
                logger.log(format!("Saving WebP file took {:?}", save_duration));

                if let Err(e) = save_result {
                    logger.log(format!("Failed to save {}: {}", output_path.display(), e));
                } else {
                    logger.log("WebP file saved successfully".to_string());

                    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);
                    let compressed_size = std::fs::metadata(&output_path).map(|m| m.len()).unwrap_or(0);
                    
                    original_sizes.lock().push(original_size);
                    compressed_sizes.lock().push(compressed_size);

                    let compression_rate = 1.0 - (compressed_size as f32 / original_size as f32);
                    
                    let mut image_details = image_details.lock();
                    if let Some(detail) = image_details.get_mut(index) {
                        detail.compressed_size = Some(compressed_size);
                        detail.compression_rate = Some(compression_rate);
                    }
                    drop(image_details);

                    sender.send(ConversionUpdate::ImageProcessed(index, compressed_size, compression_rate)).unwrap();
                }
            } else if let Err(e) = webp_result {
                logger.log(format!("Failed to encode {}: {}", input_path.display(), e));
            }
        } else {
            logger.log(format!("Failed to load {}: {}", input_path.display(), img_result.unwrap_err()));
        }

        let mut progress = progress.lock();
        progress.completed += 1;
        progress.status = format!("Converting image {} of {}", progress.completed, total_files);
        logger.log(progress.status.clone());
        logger.log(get_memory_usage());
        sender.send(ConversionUpdate::Progress(progress.completed, total_files)).unwrap();
    
        *currently_processing.lock() = None;
        
    });

    sender.send(ConversionUpdate::Completed).unwrap();

    let total_duration = start_time.elapsed();
    logger.log(format!("Conversion process completed in {:?}", total_duration));

    let mut progress = progress.lock();
    progress.status = "Conversion complete!".to_string();
}

// Wrap other image processing functions with performance measurements
fn load_image(path: &PathBuf) -> Result<DynamicImage, ImageError> {
    let (result, duration) = measure_time(|| ImageReader::open(path)?.decode());
    println!("load_image took {:?}", duration);
    result
}

fn resize_image(img: DynamicImage, width: u32, height: u32) -> DynamicImage {
    let (result, duration) = measure_time(|| img.resize_exact(width, height, image::imageops::FilterType::Lanczos3));
    println!("resize_image took {:?}", duration);
    result
}

fn encode_to_webp(img: &DynamicImage, quality: f32) -> Result<Vec<u8>, ImageError> {
    let encoder = webp::Encoder::from_image(img).map_err(|e| {
        ImageError::Encoding(image::error::EncodingError::new(
            image::error::ImageFormatHint::Exact(image::ImageFormat::WebP),
            e
        ))
    })?;
    let webp = encoder.encode(quality);
    Ok(webp.to_vec())
}

fn save_webp(webp_data: &[u8], output_path: &PathBuf) -> std::io::Result<()> {
    let (result, duration) = measure_time(|| {
        let mut file = File::create(output_path)?;
        file.write_all(webp_data)?;
        Ok(())
    });
    println!("save_webp took {:?}", duration);
    result
}
