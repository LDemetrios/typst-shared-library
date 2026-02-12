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
            .or_else(|| Some(FsPackages::new("/tmp/typst/packages")));
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

#[cfg(target_arch = "wasm32")]
pub fn load_package_file_in_memory(spec: &typst::syntax::package::PackageSpec, path: &str) -> FileResult<Vec<u8>> {
    use std::io::Read;
    use std::io::Cursor;
    use flate2::read::GzDecoder;
    use tar::Archive;
    use typst_library::diag::{FileError, PackageError};
    use typst_kit::downloader::Downloader;

    if spec.namespace != UniversePackages::NAMESPACE {
        return Err(FileError::Package(PackageError::NotFound(spec.clone())));
    }

    let url = format!(
        "https://packages.typst.org/{}/{}-{}.tar.gz",
        UniversePackages::NAMESPACE,
        spec.name,
        spec.version
    );
    let data = download::downloader()
        .download(spec, &url)
        .map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => FileError::Package(PackageError::NotFound(spec.clone())),
            _ => FileError::Package(PackageError::NetworkFailed(Some(err.to_string().into()))),
        })?;

    let mut archive = Archive::new(GzDecoder::new(Cursor::new(data)));
    let wanted = path.strip_prefix('/').unwrap_or(path);
    let mut entries = archive.entries().map_err(|err| FileError::Other(Some(err.to_string().into())))?;
    while let Some(entry) = entries.next() {
        let mut entry = entry.map_err(|err| FileError::Other(Some(err.to_string().into())))?;
        let entry_path = entry
            .path()
            .map_err(|err| FileError::Other(Some(err.to_string().into())))?;
        let entry_path = entry_path.to_string_lossy();
        if entry_path == wanted || entry_path == format!("./{}", wanted) {
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|err| FileError::Other(Some(err.to_string().into())))?;
            return Ok(buf);
        }
    }

    Err(FileError::NotFound(path.into()))
}
