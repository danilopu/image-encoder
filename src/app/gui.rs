use std::sync::mpsc::channel;
use crate::app::App;
use crate::app::file_dialogs;
use crate::app::image_processing;
use crate::app::ImageDetail;
use egui::{Color32, Frame, ProgressBar, Rounding, Slider, Stroke, RichText};

pub fn render(app: &mut App, ctx: &egui::Context) {
    let frame = Frame {
        fill: Color32::from_rgb(30, 30, 40),
        rounding: Rounding::same(10.0),
        stroke: Stroke::new(1.0, Color32::from_rgb(100, 200, 250)),
        inner_margin: egui::style::Margin::same(20.0),
        ..Default::default()
    };

    egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
        ui.heading(RichText::new("JPEG to WebP Converter").size(28.0).color(Color32::from_rgb(100, 200, 250)));
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let button_width = 200.0;
                if ui.add_sized([button_width, 30.0], egui::Button::new("Select Images")).clicked() {
                    if let Some(files) = file_dialogs::select_images() {
                        app.input_files = files.clone();
                        let image_details: Vec<ImageDetail> = files.iter().map(|path| {
                            let metadata = std::fs::metadata(path).unwrap();
                            ImageDetail {
                                name: path.file_name().unwrap().to_string_lossy().into_owned(),
                                original_size: metadata.len(),
                                compressed_size: None,
                                compression_rate: None,
                                status: "Load successful".to_string(),
                                error_message: None,
                            }
                        }).collect();
                        *app.image_details.lock() = image_details;
                        app.log_messages.lock().push(format!("[{}] Images selected successfully.", chrono::Local::now().format("%H:%M:%S")));
                    }
                }
                ui.add_space(5.0);
                if ui.add_sized([button_width, 30.0], egui::Button::new("Select Output Directory")).clicked() {
                    if let Some(dir) = file_dialogs::select_output_directory() {
                        app.output_directory = Some(dir);
                        app.log_messages.lock().push(format!("[{}] Output directory selected.", chrono::Local::now().format("%H:%M:%S")));
                    }
                }

                ui.add_space(10.0);

                // Display output directory
                ui.group(|ui| {
                    ui.set_width(button_width);
                    ui.label(RichText::new("Output Directory:").size(16.0).color(Color32::from_rgb(100, 200, 250)));
                    if let Some(dir) = &app.output_directory {
                        ui.label(dir.to_string_lossy());
                    } else {
                        ui.label("Not selected (will use input directory)");
                    }
                });

                ui.add_space(10.0);

                // Conversion Settings
                ui.group(|ui| {
                    ui.set_width(button_width);
                    ui.label(RichText::new("Conversion Settings").size(16.0).color(Color32::from_rgb(100, 200, 250)));
                    ui.add(Slider::new(&mut app.compression_quality, 1.0..=100.0).text("Quality"));
                    ui.checkbox(&mut app.resize_enabled, "Enable Resizing");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut app.width).prefix("Width: ").suffix("px"));
                        ui.add(egui::DragValue::new(&mut app.height).prefix("Height: ").suffix("px"));
                    });
                });

                ui.add_space(10.0);

                // Results
                ui.group(|ui| {
                    ui.set_width(button_width);
                    ui.label(RichText::new("Results").size(16.0).color(Color32::from_rgb(100, 200, 250)));

                    let total_files = app.input_files.len();
                    let total_original_size: f64 = app.input_files.iter()
                        .filter_map(|f| std::fs::metadata(f).ok())
                        .map(|m| m.len() as f64 / (1024.0 * 1024.0))
                        .sum();
                    
                    let total_compressed_size: f64 = app.compressed_sizes.lock().iter().sum::<u64>() as f64 / (1024.0 * 1024.0);
                    let size_reduction = if total_original_size > 0.0 {
                        (1.0 - (total_compressed_size / total_original_size)) * 100.0
                    } else {
                        0.0
                    };

                    ui.label(RichText::new(format!("Files: {}", total_files)).color(Color32::from_rgb(200, 200, 200)));
                    ui.label(RichText::new(format!("Original Size: {:.2} MB", total_original_size)).color(Color32::from_rgb(200, 200, 200)));
                    ui.label(RichText::new(format!("Compressed Size: {:.2} MB", total_compressed_size)).color(Color32::from_rgb(200, 200, 200)));
                    ui.label(RichText::new(format!("Size Reduction: {:.2}%", size_reduction)).color(Color32::from_rgb(200, 200, 200)));
                });

                ui.add_space(10.0);

                if ui.add_sized([button_width, 30.0], egui::Button::new("Start Conversion")).clicked() {
                    if app.input_files.is_empty() {
                        app.log_messages.lock().push(format!("[{}] No images selected for conversion.", chrono::Local::now().format("%H:%M:%S")));
                    } else {
                        app.log_messages.lock().push(format!("[{}] Starting conversion...", chrono::Local::now().format("%H:%M:%S")));
                        start_conversion(app);
                    }
                }
            });

            ui.add_space(10.0);

            // Selected Images (scrollable table)
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(ui.available_height() - 250.0); // Adjust this value as needed
                    ui.label(RichText::new("Selected Images:").size(16.0).color(Color32::from_rgb(100, 200, 250)));
                    
                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                        egui::Grid::new("image_details_grid")
                        .num_columns(6)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(RichText::new("#").strong());
                            ui.label(RichText::new("Name").strong());
                            ui.label(RichText::new("Original Size").strong());
                            ui.label(RichText::new("Compressed Size").strong());
                            ui.label(RichText::new("Compression Rate").strong());
                            ui.label(RichText::new("Status").strong());
                            ui.end_row();

                            let image_details = app.image_details.lock();
                            for (index, detail) in image_details.iter().enumerate() {
                                let text_color = if Some(index) == *app.currently_processing.lock() {
                                    Color32::YELLOW
                                } else {
                                    Color32::WHITE
                                };

                                ui.label(RichText::new(format!("{}", index + 1)).color(text_color));
                                ui.label(RichText::new(&detail.name).color(text_color));
                                ui.label(RichText::new(format!("{:.2} MB", detail.original_size as f64 / (1024.0 * 1024.0))).color(text_color));
                                
                                if detail.status == "Conversion failed" {
                                    ui.label(RichText::new("-").color(Color32::RED));
                                    ui.label(RichText::new("-").color(Color32::RED));
                                } else {
                                    ui.label(RichText::new(match detail.compressed_size {
                                        Some(size) => format!("{:.2} MB", size as f64 / (1024.0 * 1024.0)),
                                        None => "-".to_string(),
                                    }).color(text_color));
                                    ui.label(RichText::new(match detail.compression_rate {
                                        Some(rate) => format!("{:.2}%", rate * 100.0),
                                        None => "-".to_string(),
                                    }).color(text_color));
                                }

                                let status_color = match detail.status.as_str() {
                                    "Load successful" => Color32::GREEN,
                                    "Processing..." => Color32::YELLOW,
                                    "Conversion successful" => Color32::GREEN,
                                    "Conversion failed" => Color32::RED,
                                    _ => text_color,
                                };
                                ui.label(RichText::new(&detail.status).color(status_color));
                                ui.end_row();
                            }
                            drop(image_details);
                        });
                    });
                });
            });
        });

        ui.add_space(20.0);

        // Conversion Log with Progress Bar
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.label(RichText::new("Conversion Log").size(16.0).color(Color32::from_rgb(100, 200, 250)));

            let progress = app.conversion_progress.lock();
            if progress.total > 0 {
                let progress_ratio = progress.completed as f32 / progress.total as f32;
                ui.add(ProgressBar::new(progress_ratio).text(format!("{:.0}%", progress_ratio * 100.0)));
            }

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                let logs = app.log_messages.lock();
                for log in logs.iter() {
                    if log.contains("error") || log.contains("failed") {
                        ui.label(RichText::new(log).color(Color32::RED));
                    } else {
                        ui.label(log);
                    }
                }
            });
        });   
    });
}

fn start_conversion(app: &mut App) {
    let input_files = app.input_files.clone();
    let output_directory = app.output_directory.clone().unwrap_or_else(|| {
        input_files.first().and_then(|path| path.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap())
    });
    let resize_enabled = app.resize_enabled;
    let width = app.width;
    let height = app.height;
    let quality_enabled = app.quality_enabled;
    let compression_quality = app.compression_quality;
    let rename_enabled = app.rename_enabled;
    let output_filename = app.output_filename.clone();
    let conversion_progress = app.conversion_progress.clone();
    let log_messages = app.log_messages.clone();
    let original_sizes = app.original_sizes.clone();
    let compressed_sizes = app.compressed_sizes.clone();
    let image_details = app.image_details.clone();

    let (sender, receiver) = channel();
    app.conversion_receiver = Some(receiver);

    std::thread::spawn(move || {
        image_processing::convert_images(
            input_files,
            output_directory,
            resize_enabled,
            width,
            height,
            quality_enabled,
            compression_quality,
            rename_enabled,
            output_filename,
            conversion_progress,
            log_messages,
            original_sizes,
            compressed_sizes,
            image_details,
            sender,
        );
    });
}