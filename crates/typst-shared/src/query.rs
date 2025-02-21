use crate::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned, Resolve};
use crate::java_world::JavaWorld;
use crate::memory_management::{JavaResult, ThickBytePtr};
use serde::Serialize;
use typst::comemo::Track;
use typst::diag::{EcoString, HintedStrResult,  Warned};
use typst::foundations::{Content, IntoValue, LocatableSelector, Scope};
use typst::layout::PagedDocument;
use typst::routines::EvalMode;
use typst::syntax::Span;
use typst::utils::tick;
use typst::World;
use typst_eval::eval_string;

#[no_mangle]
pub extern "C" fn query(
    world_ptr: *mut JavaWorld,
    selector_thick: ThickBytePtr,
    fmt_type: i32,
) -> JavaResult<ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>>> {
    tick!();
    let mut world = unsafe { Box::from_raw(world_ptr) };
    tick!();
    let selector = selector_thick.to_str();
    tick!();

    // Reset everything and ensure that the main file is present.
    tick!();
    world.reset();
    // tick!();
    // world.source(world.main()).map_err(|err| err.to_string()).unwrap();

    tick!();
    let Warned { output, warnings } = typst::compile(&world);

    tick!();
    let serialized = output
        .map(|it| {
            let data = retrieve(&world, selector.as_ref(), &it).unwrap();
            format(data, fmt_type)
        })
        .map_err(|it| it.resolve(world.as_ref()));

    tick!();
    let result: ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>> =
        ExtendedWarned {
            output: serialized,
            warnings: warnings.resolve(world.as_ref()),
        };

    tick!("{:?}", result);

    let _ = Box::into_raw(world); // Not to drop the world!

    JavaResult::pack(result)
}

/// Retrieve the matches for the selector.
fn retrieve(
    world: &dyn World,
    selector: &str,
    document: &PagedDocument,
) -> HintedStrResult<Vec<Content>> {
    let selector = eval_string(
        &typst::ROUTINES,
        world.track(),
        selector,
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
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

/// Format the query result in the output format.
fn format(elements: Vec<Content>, fmt_type: i32) -> String {
    let mapped: Vec<_> =
        elements.into_iter().filter_map(|c| Some(c.into_value())).collect();

    serialize(&mapped, fmt_type)
}

/// Serialize data to the output format.
fn serialize(data: &impl Serialize, fmt_type: i32) -> String {
    match fmt_type {
        0 => serde_json::to_string_pretty(data).expect("Unexpected error in serializing"),
        1 => serde_json::to_string(data).expect("Unexpected error in serializing"),
        2 => serde_yaml::to_string(data).expect("Unexpected error in serializing"),
        _ => panic!("Unexpected tag {} for fmt_type", fmt_type),
    }
}
