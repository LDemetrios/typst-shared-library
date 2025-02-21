use std::ops::Range;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use typst::diag::{
    EcoString, FileError, PackageError, Severity, SourceDiagnostic, Tracepoint, Warned,
};
use typst::ecow::EcoVec;
use typst::syntax::package::{PackageSpec, PackageVersion};
use typst::syntax::{FileId, Span, Spanned, VirtualPath};
use typst::World;

pub trait Resolve<Output> {
    fn resolve(self, world: &dyn World) -> Output;
}

macro_rules! resolve_via_into {
    ($from: ty, $to : ty) => {
        impl Resolve<$to> for $from {
            fn resolve(self, _world: &dyn World) -> $to {
                self.into()
            }
        }
    };
}

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct ExtendedFileDescriptor {
    pub pack: Option<ExtendedPackageSpec>,
    pub path: String,
}

impl From<FileId> for ExtendedFileDescriptor {
    fn from(value: FileId) -> Self {
        ExtendedFileDescriptor {
            pack: FileId::package(&value).map(|it| it.clone().into()),
            path: FileId::vpath(&value)
                .as_rooted_path()
                .to_str()
                .map(|it| it.to_string())
                .unwrap(),
        }
    }
}

impl From<ExtendedFileDescriptor> for FileId {
    fn from(value: ExtendedFileDescriptor) -> Self {
        FileId::new(value.pack.map(|it| it.into()), VirtualPath::new(PathBuf::from(value.path)))
    }
}

resolve_via_into!(FileId, ExtendedFileDescriptor);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ExtendedSourceDiagnostic {
    pub severity: Severity,
    pub span: ExtendedSpan,
    pub message: String,
    pub trace: Vec<ExtendedSpanned<ExtendedTracepoint>>,
    pub hints: Vec<String>,
}

impl Resolve<ExtendedSourceDiagnostic> for SourceDiagnostic {
    fn resolve(self, world: &dyn World) -> ExtendedSourceDiagnostic {
        ExtendedSourceDiagnostic {
            severity: self.severity,
            span: self.span.resolve(world),
            message: self.message.to_string(),
            trace: self.trace.into_iter().map(|s| s.resolve(world)).collect(),
            hints: self.hints.into_iter().map(|e| e.to_string()).collect(),
        }
    }
}

// impl From<SourceDiagnostic> for ExtendedSourceDiagnostic {
//     fn from(diagnostic: SourceDiagnostic) -> Self {
//         ExtendedSourceDiagnostic {
//             severity: diagnostic.severity,
//             span: diagnostic.span.into(),
//             message: diagnostic.message.to_string(),
//             trace:,
//             hints:,
//         }
//     }
// }
//
// impl From<ExtendedSourceDiagnostic> for SourceDiagnostic {
//     fn from(diagnostic: ExtendedSourceDiagnostic) -> Self {
//         SourceDiagnostic {
//             severity: diagnostic.severity,
//             span: diagnostic.span.into(),
//             message: EcoString::from(diagnostic.message),
//             trace: diagnostic.trace.into_iter().map(Spanned::from).collect(),
//             hints: diagnostic.hints.into_iter().map(EcoString::from).collect(),
//         }
//     }
// }

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct ExtendedSpanned<T> {
    pub v: T,
    pub span: ExtendedSpan,
}

impl<T2, T: Resolve<T2>> Resolve<ExtendedSpanned<T2>> for Spanned<T> {
    fn resolve(self, world: &dyn World) -> ExtendedSpanned<T2> {
        ExtendedSpanned {
            v: self.v.resolve(world),
            span: self.span.resolve(world),
        }
    }
}

// impl<T2, T1: Into<T2>> From<Spanned<T1>> for ExtendedSpanned<T2> {
//     fn from(spanned: Spanned<T1>) -> Self {
//         ExtendedSpanned { v: spanned.v.into(), span: spanned.span.into() }
//     }
// }
//
// impl<T2, T1: Into<T2>> From<ExtendedSpanned<T1>> for Spanned<T2> {
//     fn from(spanned: ExtendedSpanned<T1>) -> Self {
//         Spanned { v: spanned.v.into(), span: spanned.span.into() }
//     }
// }

#[derive(Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize)]
pub struct ExtendedSpan {
    pub native: u64,
    pub file: Option<ExtendedFileDescriptor>,
    pub start_ind: i64,
    pub end_ind: i64,
    pub start_line: i64,
    pub start_col: i64,
    pub end_line: i64,
    pub end_col: i64,
}

impl Resolve<ExtendedSpan> for Span {
    fn resolve(self, world: &dyn World) -> ExtendedSpan {
        let native = self.into_raw().get();
        let file = self.id().map(|it| it.into());
        let (start_ind, end_ind, start_line, start_col, end_line, end_col) =
            resolve_range(self, world).unwrap_or((0, 0, 0, 0, 0, 0));
        ExtendedSpan {
            native,
            file,
            start_ind,
            end_ind,
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

fn resolve_range(
    span: Span,
    world: &dyn World,
) -> Option<(i64, i64, i64, i64, i64, i64)> {
    let src = world.source(span.id()?).ok()?;
    let Range { start, end } = src.range(span)?;
    let start_line = src.byte_to_line(start).map_or(-1, |it| it as i64);
    let start_col = src.byte_to_column(start).map_or(-1, |it| it as i64);
    let end_line = src.byte_to_line(end).map_or(-1, |it| it as i64);
    let end_col = src.byte_to_column(end).map_or(-1, |it| it as i64);
    Some((start as i64, end as i64, start_line, start_col, end_line, end_col))
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtendedTracepoint {
    Call { function: Option<EcoString> },
    Show { string: EcoString },
    Import,
}

impl From<Tracepoint> for ExtendedTracepoint {
    fn from(tracepoint: Tracepoint) -> Self {
        match tracepoint {
            Tracepoint::Call(function) => ExtendedTracepoint::Call { function },
            Tracepoint::Show(string) => ExtendedTracepoint::Show { string },
            Tracepoint::Import => ExtendedTracepoint::Import,
        }
    }
}

impl From<ExtendedTracepoint> for Tracepoint {
    fn from(tracepoint: ExtendedTracepoint) -> Self {
        match tracepoint {
            ExtendedTracepoint::Call { function } => Tracepoint::Call(function),
            ExtendedTracepoint::Show { string } => Tracepoint::Show(string),
            ExtendedTracepoint::Import => Tracepoint::Import,
        }
    }
}

resolve_via_into!(Tracepoint, ExtendedTracepoint);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ExtendedWarned<T> {
    pub output: T,
    pub warnings: Vec<ExtendedSourceDiagnostic>,
}

impl<T2, T: Resolve<T2>> Resolve<ExtendedWarned<T2>> for Warned<T> {
    fn resolve(self, world: &dyn World) -> ExtendedWarned<T2> {
        ExtendedWarned {
            output: self.output.resolve(world),
            warnings: self.warnings.into_iter().map(|it| it.resolve(world)).collect(),
        }
    }
}

//
// impl<T2, T1: Into<T2>> From<Warned<T1>> for ExtendedWarned<T2> {
//     fn from(warned: Warned<T1>) -> Self {
//         ExtendedWarned {
//             output: warned.output.into(),
//             warnings: warned
//                 .warnings
//                 .into_iter()
//                 .map(ExtendedSourceDiagnostic::from)
//                 .collect(),
//         }
//     }
// }
//
// impl<T2, T1: Into<T2>> From<ExtendedWarned<T1>> for Warned<T2> {
//     fn from(warned: ExtendedWarned<T1>) -> Self {
//         Warned {
//             output: warned.output.into(),
//             warnings: warned.warnings.into_iter().map(SourceDiagnostic::from).collect(),
//         }
//     }
// }

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtendedFileError {
    NotFound { path: String },
    AccessDenied,
    IsDirectory,
    NotSource,
    InvalidUtf8,
    Package { error: ExtendedPackageError },
    Other { message: Option<EcoString> },
}

impl From<FileError> for ExtendedFileError {
    fn from(error: FileError) -> Self {
        match error {
            FileError::NotFound(path) => {
                ExtendedFileError::NotFound { path: path.to_string_lossy().into_owned() }
            }
            FileError::AccessDenied => ExtendedFileError::AccessDenied,
            FileError::IsDirectory => ExtendedFileError::IsDirectory,
            FileError::NotSource => ExtendedFileError::NotSource,
            FileError::InvalidUtf8 => ExtendedFileError::InvalidUtf8,
            FileError::Package(error) => {
                ExtendedFileError::Package { error: error.into() }
            }
            FileError::Other(message) => ExtendedFileError::Other { message },
        }
    }
}

impl From<ExtendedFileError> for FileError {
    fn from(error: ExtendedFileError) -> Self {
        match error {
            ExtendedFileError::NotFound { path } => {
                FileError::NotFound(PathBuf::from(path))
            }
            ExtendedFileError::AccessDenied => FileError::AccessDenied,
            ExtendedFileError::IsDirectory => FileError::IsDirectory,
            ExtendedFileError::NotSource => FileError::NotSource,
            ExtendedFileError::InvalidUtf8 => FileError::InvalidUtf8,
            ExtendedFileError::Package { error } => FileError::Package(error.into()),
            ExtendedFileError::Other { message } => FileError::Other(message),
        }
    }
}

resolve_via_into!(FileError, ExtendedFileError);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExtendedPackageError {
    NotFound { package: ExtendedPackageSpec },
    VersionNotFound { package: PackageSpec, version: ExtendedPackageVersion },
    NetworkFailed { message: Option<String> },
    MalformedArchive { message: Option<String> },
    Other { message: Option<String> },
}

impl From<PackageError> for ExtendedPackageError {
    fn from(error: PackageError) -> Self {
        match error {
            PackageError::NotFound(package) => {
                ExtendedPackageError::NotFound { package: package.into() }
            }
            PackageError::VersionNotFound(package, version) => {
                ExtendedPackageError::VersionNotFound { package, version: version.into() }
            }
            PackageError::NetworkFailed(message) => ExtendedPackageError::NetworkFailed {
                message: message.map(|e| e.to_string()),
            },
            PackageError::MalformedArchive(message) => {
                ExtendedPackageError::MalformedArchive {
                    message: message.map(|e| e.to_string()),
                }
            }
            PackageError::Other(message) => {
                ExtendedPackageError::Other { message: message.map(|e| e.to_string()) }
            }
        }
    }
}

impl From<ExtendedPackageError> for PackageError {
    fn from(error: ExtendedPackageError) -> Self {
        match error {
            ExtendedPackageError::NotFound { package } => {
                PackageError::NotFound(package.into())
            }
            ExtendedPackageError::VersionNotFound { package, version } => {
                PackageError::VersionNotFound(package, version.into())
            }
            ExtendedPackageError::NetworkFailed { message } => {
                PackageError::NetworkFailed(message.map(EcoString::from))
            }
            ExtendedPackageError::MalformedArchive { message } => {
                PackageError::MalformedArchive(message.map(EcoString::from))
            }
            ExtendedPackageError::Other { message } => {
                PackageError::Other(message.map(EcoString::from))
            }
        }
    }
}

resolve_via_into!(PackageError, ExtendedPackageError);

impl<T2, E2, T1: Resolve<T2>, E1: Resolve<E2>> Resolve<Result<T2, E2>>
    for Result<T1, E1>
{
    fn resolve(self, world: &dyn World) -> Result<T2, E2> {
        match self {
            Ok(t1) => Ok(t1.resolve(world)),
            Err(e1) => Err(e1.resolve(world)),
        }
    }
}

impl<T2, T: Resolve<T2> + Clone> Resolve<Vec<T2>> for EcoVec<T> {
    fn resolve(self, world: &dyn World) -> Vec<T2> {
        self.into_iter().map(|it| it.resolve(world)).collect()
    }
}


pub type ExtendedFileResult<T> = Result<T, ExtendedFileError>;

/// A package's version.
#[derive(
    Debug,
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize
)]
pub struct ExtendedPackageVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct ExtendedPackageSpec {
    pub namespace: String,
    pub name: String,
    pub version: ExtendedPackageVersion,
}

impl From<PackageVersion> for ExtendedPackageVersion {
    fn from(value: PackageVersion) -> Self {
        ExtendedPackageVersion {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
        }
    }
}

impl From<ExtendedPackageVersion> for PackageVersion {
    fn from(value: ExtendedPackageVersion) -> Self {
        PackageVersion {
            major: value.major,
            minor: value.minor,
            patch: value.patch,
        }
    }
}

impl From<PackageSpec> for ExtendedPackageSpec {
    fn from(value: PackageSpec) -> Self {
        ExtendedPackageSpec {
            namespace: value.namespace.to_string(),
            name: value.name.to_string(),
            version: value.version.into(),
        }
    }
}

impl From<ExtendedPackageSpec> for PackageSpec {
    fn from(value: ExtendedPackageSpec) -> Self {
        PackageSpec {
            namespace: EcoString::from(value.namespace),
            name: EcoString::from(value.name),
            version: value.version.into(),
        }
    }
}

impl<T2, T1: Resolve<T2>> Resolve<Option<T2>> for Option<T1> {
    fn resolve(self, world: &dyn World) -> Option<T2> {
        match self {
            None => None,
            Some(x) => Some(x.resolve(world)),
        }
    }
}
