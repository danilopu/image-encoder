// file_dialogs.rs
use rfd::FileDialog;
use std::path::PathBuf;

pub fn select_images() -> Option<Vec<PathBuf>> {
    FileDialog::new()
        .add_filter("Image", &["jpg", "jpeg", "png"])
        .pick_files()
}

pub fn select_output_directory() -> Option<PathBuf> {
    FileDialog::new().pick_folder()
}
