// Added by LDemetrios

use rustc_hash::FxHashSet;
use typst::comemo::Track;
use typst::ecow::EcoVec;
use typst::engine::{Route, Sink, Traced};
use typst::foundations::Value;
use typst::utils::LazyHash;
use typst::World;
use typst_library::Library;
use crate::compile::diagnostics_from_message;
use crate::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned, Resolve};
use typst_library::foundations::raw_string::RawString;
use crate::raw_time::RawNow;
use crate::utils::to_file_id;
use crate::values::ToJson;
use crate::world::{CompositeWorld, FilesCache, FontCollection};
use crate::world_parts::TicketedReader;

#[unsafe(no_mangle)]
pub extern "C" fn eval_main(
    session: i64,
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    stdlib: &mut LazyHash<Library>,
    fonts: &mut FontCollection,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
) {
    let main = RawString::new(main_len, main_ptr);
    let now = RawNow::new(now_millis_or_flag, now_nanos);
    let world = CompositeWorld::<TicketedReader>::new(
        Some(context),
        Some(fonts),
        Some(stdlib),
        Some(to_file_id(main)),
        now.resolve(),
        session
    );

    let mut sink = Sink::new();
    let output = eval_impl(&world, &mut sink);

    let result_v: ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>> =
        ExtendedWarned {
            output: output.map(|content| content.to_json().to_string()),
            warnings: sink.warnings().resolve(&world),
        };

    RawString::write_from(result, &result_v);
}

fn eval_impl(
    world: &dyn World,
    sink: &mut Sink,
) -> Result<typst::foundations::Content, Vec<ExtendedSourceDiagnostic>> {
    let main = world.main();
    let main = world.source(main).map_err(|_| {
        diagnostics_from_message(format!("Missing main file {}", main.resolve(world)))
    })?;

    typst_eval::eval(
        &typst::ROUTINES,
        world.track(),
        Traced::default().track(),
        sink.track_mut(),
        Route::default().track(),
        &main,
    )
    .map(|module| module.content())
    .map_err(deduplicate)
    .map_err(|it| it.resolve(world))
}

fn deduplicate(
    mut diags: EcoVec<typst::diag::SourceDiagnostic>,
) -> EcoVec<typst::diag::SourceDiagnostic> {
    let mut unique = FxHashSet::default();
    diags.retain(|diag| {
        let hash = typst::utils::hash128(&(&diag.span, &diag.message));
        unique.insert(hash)
    });
    diags
}

impl Resolve<Value> for Value {
    fn resolve(self, _world: &dyn World) -> Value {
        self
    }
}
