// Added by LDemetrios

use std::path::PathBuf;
use typst::syntax::{FileId, VirtualRoot};
use typst_kit::files::FsRoot;
use typst_kit::packages::{FsPackages, SystemPackages, UniversePackages};
use typst_library::diag::FileResult;

pub mod download;

pub struct PackageManager(SystemPackages);

impl PackageManager {
    pub fn new(
        package_cache_path: Option<PathBuf>,
        package_path: Option<PathBuf>,
    ) -> Self {
        let cache = package_cache_path
            .map(FsPackages::new)
            .or_else(FsPackages::system_cache)
            .or_else(|| Some(FsPackages::new("/packages")));
        let data = package_path.map(FsPackages::new).or_else(FsPackages::system_data);
        Self(SystemPackages::from_parts(
            data,
            cache,
            UniversePackages::new(download::downloader()),
        ))
    }

    pub fn of(
        package_cache_path: Option<String>,
        package_path: Option<String>,
    ) -> Self {
        Self::new(
            package_cache_path.map(|s| PathBuf::from(s)),
            package_path.map(|s| PathBuf::from(s)),
        )
    }

    pub fn load(&self, id: FileId) -> FileResult<Vec<u8>> {
        let spec = match id.root() {
            VirtualRoot::Package(spec) => spec,
            VirtualRoot::Project => unreachable!("package load called for project file"),
        };
        let root: FsRoot = self.0.obtain(spec)?;
        root.load(id.vpath()).map(|bytes| bytes.into_vec())
    }
}
