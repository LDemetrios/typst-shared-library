// Added by LDemetrios

use std::path::Path;
use typst::utils::LazyHash;
use typst_kit::fonts::{self, FontPath, FontStore};
use typst_library::text::{Font, FontBook, FontInfo};

pub struct FontCollection {
    store: FontStore,
}

impl Default for FontCollection {
    fn default() -> Self {
        Self::new(true, true, vec![])
    }
}

impl FontCollection {
    pub fn new(include_system: bool, include_embedded: bool, paths: Vec<String>) -> Self {
        let mut fonts = FontStore::new();

        if include_system {
            fonts.extend(fonts::system());
        }

        if include_embedded {
            fonts.extend(fonts::embedded());
        }

        for path in &paths {
            let mut any_scanned = false;
            let mut scanned = Vec::new();
            #[allow(unused_assignments)]
            {
                scanned.extend(fonts::scan(path.as_ref()));
            }
            if !scanned.is_empty() {
                any_scanned = true;
                fonts.extend(scanned);
            }

            if !any_scanned {
                add_fonts_from_path(&mut fonts, Path::new(path));
            }
        }

        Self { store: fonts }
    }

    pub fn book(&self) -> &LazyHash<FontBook> {
        self.store.book()
    }

    pub fn font(&self, index: usize) -> Option<Font> {
        self.store.font(index)
    }
}

fn add_fonts_from_path(fonts: &mut FontStore, path: &Path) {
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    add_fonts_from_path(fonts, &entry.path());
                }
            }
        } else if metadata.is_file() {
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
            if ext == "ttf" || ext == "otf" || ext == "ttc" || ext == "otc" {
                if let Ok(data) = std::fs::read(path) {
                    for (index, info) in FontInfo::iter(&data).enumerate() {
                        fonts.push((
                            FontPath { path: path.to_path_buf(), index: index as u32 },
                            info,
                        ));
                    }
                }
            }
        }
    }
}
