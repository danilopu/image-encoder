// app.rs
pub mod gui;
pub mod image_processing;
pub mod file_dialogs;

use eframe::egui;
use eframe::App as EframeApp;
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Instant;
use std::sync::mpsc::Receiver;


pub struct App {
    // Application state
    pub input_files: Vec<PathBuf>,
    pub output_directory: Option<PathBuf>,
    pub resize_enabled: bool,
    pub width: u32,
    pub height: u32,
    pub compression_quality: f32,
    pub output_filename: String,
    pub quality_enabled: bool,
    pub rename_enabled: bool,
    pub conversion_progress: Arc<Mutex<ConversionProgress>>,
    pub log_messages: Arc<Mutex<Vec<String>>>,
    pub selected_image: Option<PathBuf>,
    pub conversion_start_time: Option<Instant>,
    pub original_size: Option<u64>,
    pub compressed_size: Option<u64>,
    pub original_sizes: Arc<Mutex<Vec<u64>>>,
    pub compressed_sizes: Arc<Mutex<Vec<u64>>>,
    pub original_width: u32,
    pub original_height: u32,
    pub image_details: Arc<Mutex<Vec<ImageDetail>>>,
    pub currently_processing: Arc<Mutex<Option<usize>>>,
    pub conversion_receiver: Option<Receiver<ConversionUpdate>>,
}

#[derive(Clone)]
pub enum ConversionUpdate {
    Progress(usize, usize),  // (completed, total)
    ImageProcessed(usize, Option<u64>, Option<f32>),  // (index, compressed_size, compression_rate)
    Completed,
    StatusUpdate(usize, String, Option<String>),  // (index, status, error_message)
    ResultsUpdate(f64, f64),  // (total_original, total_compressed)
}

pub struct ConversionProgress {
    pub total: usize,
    pub completed: usize,
    pub status: String,
}

#[derive(Clone, Debug)]
pub struct ImageDetail {
    pub name: String,
    pub original_size: u64,
    pub compressed_size: Option<u64>,
    pub compression_rate: Option<f32>,
    pub status: String,
    pub error_message: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input_files: Vec::new(),
            output_directory: None,
            resize_enabled: false,
            width: 800,
            height: 600,
            compression_quality: 80.0,
            output_filename: String::from("output"),
            quality_enabled: false,
            rename_enabled: false,
            conversion_progress: Arc::new(Mutex::new(ConversionProgress {
                total: 0,
                completed: 0,
                status: String::new(),
            })),
            log_messages: Arc::new(Mutex::new(Vec::new())),
            selected_image: None,
            conversion_start_time: None,
            original_size: None,
            compressed_size: None,
            original_sizes: Arc::new(Mutex::new(Vec::new())),
            compressed_sizes: Arc::new(Mutex::new(Vec::new())),
            original_width: 0,
            original_height: 0,
            image_details: Arc::new(Mutex::new(Vec::new())),
            currently_processing: Arc::new(Mutex::new(None)),
            conversion_receiver: None,
        }
    }
}

impl EframeApp for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut completed = false;
        let mut needs_redraw = false;

        if let Some(receiver) = &self.conversion_receiver {
            while let Ok(update) = receiver.try_recv() {
                match update {
                    ConversionUpdate::Progress(completed, total) => {
                        let mut progress = self.conversion_progress.lock();
                        progress.completed = completed;
                        progress.total = total;
                        drop(progress); // Release the lock as soon as possible
                        needs_redraw = true;
                    }
                    ConversionUpdate::ImageProcessed(index, compressed_size, compression_rate) => {
                        let mut image_details = self.image_details.lock();
                        if let Some(detail) = image_details.get_mut(index) {
                            detail.compressed_size = compressed_size;
                            detail.compression_rate = compression_rate;
                        }
                        drop(image_details); // Release the lock as soon as possible
                        needs_redraw = true;
                    }
                    ConversionUpdate::StatusUpdate(index, status, error_message) => {
                        let mut image_details = self.image_details.lock();
                        if let Some(detail) = image_details.get_mut(index) {
                            detail.status = status;
                            detail.error_message = error_message;
                        }
                        drop(image_details); // Release the lock as soon as possible
                        needs_redraw = true;
                    }
                    ConversionUpdate::ResultsUpdate(total_original, total_compressed) => {
                        self.original_size = Some(total_original as u64);
                        self.compressed_size = Some(total_compressed as u64);
                        needs_redraw = true;
                    }
                    ConversionUpdate::Completed => {
                        completed = true;
                        needs_redraw = true;
                    }
                }
            }
        }

        if completed {
            self.conversion_receiver = None;
        }

        // Render the GUI
        gui::render(self, ctx);

        // Force a redraw if needed
        if needs_redraw {
            ctx.request_repaint();
        }
    }
}
