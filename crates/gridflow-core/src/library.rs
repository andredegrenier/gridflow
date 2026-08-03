//! Global class library. Documents opt in with `use <name>`, which loads
//! `<name>.gfd` from the library root (the app passes ~/.config/gridflow/lib).
//! Only variable assignments and class definitions are imported from library
//! files; node/edge statements in them are ignored.

use std::path::{Path, PathBuf};

pub trait LibraryProvider {
    /// Source text of the library file `<name>.gfd`, if it exists.
    fn load(&self, name: &str) -> Option<String>;
    /// Names available for `use` (without extension), for the library browser.
    fn list(&self) -> Vec<String>;
}

/// Provider used when no library root is configured (tests, headless).
pub struct NoLibrary;

impl LibraryProvider for NoLibrary {
    fn load(&self, _name: &str) -> Option<String> {
        None
    }
    fn list(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct FsLibrary {
    root: PathBuf,
    /// Directory of the open document; `use "./x.gfd"` resolves against it.
    doc_dir: Option<PathBuf>,
}

impl FsLibrary {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        FsLibrary { root: root.into(), doc_dir: None }
    }

    pub fn with_doc_dir(root: impl Into<PathBuf>, doc_dir: Option<PathBuf>) -> Self {
        FsLibrary { root: root.into(), doc_dir }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file_path(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.gfd"))
    }

    /// Append a class definition's source text to a library file, creating the
    /// file (and the library root) if needed.
    pub fn append_class(&self, file: &str, class_src: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        let path = self.file_path(file);
        let mut existing = std::fs::read_to_string(&path).unwrap_or_default();
        if !existing.is_empty() && !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(class_src.trim_end());
        existing.push('\n');
        std::fs::write(&path, existing)
    }
}

impl LibraryProvider for FsLibrary {
    fn load(&self, name: &str) -> Option<String> {
        // Two forms: a bare identifier (`use people`) loads <root>/people.gfd;
        // a quoted path (`use "./styles.gfd"`) resolves relative to the
        // document's directory, making shared/checked-in diagrams portable.
        if name.contains('/') || name.contains('\\') || name.ends_with(".gfd") {
            let p = Path::new(name);
            let path = if p.is_absolute() {
                p.to_path_buf()
            } else {
                self.doc_dir.as_ref()?.join(p)
            };
            std::fs::read_to_string(path).ok()
        } else {
            std::fs::read_to_string(self.file_path(name)).ok()
        }
    }

    fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.root)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "gfd") {
                    p.file_stem().map(|s| s.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();
        names.sort();
        names
    }
}
