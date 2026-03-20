// Added by LDemetrios

use typst_library::foundations::raw_string::RawString;
use crate::serial::extended_info::ExtendedFileDescriptor;
use typst::comemo;
use typst::comemo::Track;
use typst::ecow::EcoVec;
use typst::syntax::{FileId, RootedPath, Span, SyntaxMode, VirtualPath, VirtualRoot};
use typst_eval::eval_string;
use typst_library::World;
use typst_library::diag::{SourceDiagnostic, Warned};
use typst_library::engine::Sink;
use typst_library::foundations::{Context, Scope, Value};
use typst_library::introspection::Introspector;

pub(crate) fn evaluate(
    world: &dyn World,
    source: &str,
    mode: SyntaxMode,
) -> Result<Value, EcoVec<SourceDiagnostic>> {
    eval_string(
        &typst::ROUTINES,
        world.track(),
        Sink::new().track_mut(),
        Introspector::default().track(),
        Context::none().track(),
        source,
        Span::detached(),
        mode,
        Scope::default(),
    )
}

pub(crate) fn evaluate_warned(
    world: &dyn World,
    source: &str,
    mode: SyntaxMode,
) -> Warned<Result<Value, EcoVec<SourceDiagnostic>>> {
    let mut sink = Sink::new();
    let output = eval_string(
        &typst::ROUTINES,
        world.track(),
        sink.track_mut(),
        Introspector::default().track(),
        Context::none().track(),
        source,
        Span::detached(),
        mode,
        Scope::default(),
    );
    let warnings = sink.warnings();
    Warned { output, warnings }
}

pub(crate) fn to_file_id(id: RawString) -> FileId {
    let pair = id.read_to::<ExtendedFileDescriptor>();
    let vpath = VirtualPath::new(pair.path).expect("invalid virtual path");
    let root = match pair.pack {
        Some(pack) => VirtualRoot::Package(pack.into()),
        None => VirtualRoot::Project,
    };
    FileId::new(RootedPath::new(root, vpath))
}

pub(crate) fn to_file_id_or_none(id: RawString) -> Option<FileId> {
    let pair = id.read_to::<ExtendedFileDescriptor>();
    let vpath = VirtualPath::new(pair.path).ok()?;
    let root = match pair.pack {
        Some(pack) => VirtualRoot::Package(pack.into()),
        None => VirtualRoot::Project,
    };
    Some(FileId::new(RootedPath::new(root, vpath)))
}

#[unsafe(no_mangle)]
pub extern "C" fn evict_cache(max_age: i64) {
    comemo::evict(max_age as usize)
}
