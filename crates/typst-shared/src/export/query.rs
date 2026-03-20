// Added by LDemetrios

use typst_library::foundations::raw_string::RawString;
use crate::export::raw_time::RawNow;
use crate::export::utils::{evaluate, to_file_id};
use crate::export::world_parts::TicketedReader;
use crate::serial::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned, Resolve};
use crate::values::ToJson;
use crate::world::{CompositeWorld, FilesCache, FontCollection};
use typst::World;
use typst::diag::{EcoString, HintedStrResult};
use typst::ecow::eco_vec;
use typst::foundations::{Content, IntoValue, LocatableSelector};
use typst::layout::PagedDocument;
use typst::syntax::{Span, Spanned, SyntaxMode};
use typst::utils::LazyHash;
use typst_library::Library;
use typst_library::diag::{Severity, SourceDiagnostic};

#[unsafe(no_mangle)]
pub extern "C" fn query(
    session: i64,
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
    selector_len: u64,
    selector_ptr: *mut u8,
) {
    let main = RawString::new(main_len, main_ptr);
    let now = RawNow::new(now_millis_or_flag, now_nanos);
    let selector = RawString::new(selector_len, selector_ptr);

    let world = CompositeWorld::new(
        Some(context),
        Some(fonts),
        Some(stdlib),
        Some(to_file_id(main)),
        now.resolve(),
        session
    );

    let selector = selector.into_string().unwrap();

    let result_v: ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>> =
        typst::compile(&world)
            .map(|it| {
                it.and_then(|it| match retrieve(&&world, selector.as_str(), &&it) {
                    Ok(data) => {
                        let mapped = data
                            .into_iter()
                            .map(|c| c.into_value())
                            .collect::<Vec<_>>()
                            .to_json();
                        Ok(serde_json::to_string_pretty(&mapped)
                            .expect("Unexpected error in serializing"))
                    }
                    Err(err) => Err(eco_vec!(SourceDiagnostic {
                        severity: Severity::Error,
                        span: Span::detached(),
                        message: err.message().clone(),
                        trace: Default::default(),
                        hints: err
                            .hints()
                            .iter()
                            .map(|it| Spanned::detached(it.clone()))
                            .collect(),
                    })),
                })
            })
            .resolve(&world);

    RawString::write_from(result, &result_v);
}

impl Resolve<String> for String {
    fn resolve(self, _world: &dyn World) -> String {
        self
    }
}

/// Retrieve the matches for the selector.
fn retrieve(
    world: &dyn World,
    selector: &str,
    document: &PagedDocument,
) -> HintedStrResult<Vec<Content>> {
    let selector = evaluate(world, selector, SyntaxMode::Code)
        .map_err(|errors| {
            let mut message = EcoString::from("failed to evaluate selector");
            for (i, error) in errors.into_iter().enumerate() {
                message.push_str(if i == 0 { ": " } else { ", " });
                message.push_str(&error.message);
            }
            message
        })?
        .cast::<LocatableSelector>()?;

    Ok(document
        .introspector
        .query(&selector.0)
        .into_iter()
        .collect::<Vec<_>>())
}
