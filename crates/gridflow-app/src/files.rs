//! File open/save and recents. The `.gfd` text is the sole source of truth.

use gridflow_core::document::Document;
use gridflow_core::library::FsLibrary;
use std::path::{Path, PathBuf};

pub fn library_root() -> PathBuf {
    directories::ProjectDirs::from("", "", "gridflow")
        .map(|d| d.config_dir().join("lib"))
        .unwrap_or_else(|| PathBuf::from(".gridflow-lib"))
}

pub fn new_library(doc_dir: Option<PathBuf>) -> Box<FsLibrary> {
    Box::new(FsLibrary::with_doc_dir(library_root(), doc_dir))
}

/// First-run niceness: install a few starter libraries so `use flow`,
/// `use people`, `use aws` work out of the box. Never overwrites user files.
pub fn seed_library() {
    let root = library_root();
    let starters: &[(&str, &str)] = &[
        ("flow", include_str!("../starter-lib/flow.gfd")),
        ("people", include_str!("../starter-lib/people.gfd")),
        ("aws", include_str!("../starter-lib/aws.gfd")),
    ];
    if std::fs::create_dir_all(&root).is_err() {
        return;
    }
    for (name, content) in starters {
        let path = root.join(format!("{name}.gfd"));
        if !path.exists() {
            let _ = std::fs::write(path, content);
        }
    }
}

pub fn open_document(path: &Path) -> std::io::Result<Document> {
    let text = std::fs::read_to_string(path)?;
    let doc_dir = path.parent().map(|p| p.to_path_buf());
    Ok(Document::new(text, new_library(doc_dir)))
}

pub fn save_document(doc: &mut Document, path: &Path) -> std::io::Result<()> {
    std::fs::write(path, doc.text())?;
    doc.mark_saved();
    Ok(())
}

pub fn pick_open() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("gridflow diagram", &["gfd"])
        .pick_file()
}

pub fn pick_save() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("gridflow diagram", &["gfd"])
        .set_file_name("diagram.gfd")
        .save_file()
}

pub fn push_recent(recents: &mut Vec<PathBuf>, path: &Path) {
    recents.retain(|p| p != path);
    recents.insert(0, path.to_path_buf());
    recents.truncate(10);
}
