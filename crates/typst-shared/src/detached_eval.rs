use std::mem;
use crate::extended_info::{ExtendedSourceDiagnostic, Resolve};
use crate::java_world::JavaWorld;
use crate::memory_management::{JavaResult, ThickBytePtr};
use typst::comemo::Track;
use typst::diag::SourceDiagnostic;
use typst::ecow::EcoVec;
use typst::foundations::{Scope, Value};
use typst::routines::EvalMode;
use typst::syntax::Span;
use typst::utils::tick;
use typst::World;
use typst_eval::eval_string;

#[no_mangle]
pub extern "C" fn detached_eval(
    world_ptr: *mut JavaWorld,
    source_ptr: ThickBytePtr,
) -> JavaResult<Result<String, Vec<ExtendedSourceDiagnostic>>> {
    tick!();
    let mut world = unsafe { Box::from_raw(world_ptr) };

    tick!();
    world.reset();
    tick!();
    let source = source_ptr.to_str();
    tick!();
    let result = eval(world.as_ref(), source.as_str())
        .map_err(|it| it.resolve(world.as_ref()))
        .map(|it| serde_json::to_string(&it).unwrap());

    tick!();
    let _ = Box::into_raw(world); // Not to drop the world!
    tick!();

    mem::forget(source);
    JavaResult::pack(result)
}

impl Resolve<Value> for Value {
    fn resolve(self, _world: &dyn World) -> Value {
        self
    }
}

fn eval(world: &dyn World, source: &str) -> Result<Value, EcoVec<SourceDiagnostic>> {
    eval_string(
        &typst::ROUTINES,
        world.track(),
        source,
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
}
