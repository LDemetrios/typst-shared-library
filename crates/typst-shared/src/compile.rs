use crate::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned, Resolve};
use crate::java_world::JavaWorld;
use crate::memory_management::{Base16ByteArray, JavaResult};
use chrono::{Datelike, Timelike};
use serde::Serialize;
use typst::diag::{eco_format, Warned};
use typst::foundations::Datetime;
use typst::html::HtmlDocument;
use typst::layout::{Page, PagedDocument};
use typst::utils::tick;

#[no_mangle]
pub extern "C" fn compile_html(
    world_ptr: *mut JavaWorld,
) -> JavaResult<ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>>> {
    let world = unsafe { Box::from_raw(world_ptr) };
    let Warned { output, warnings } = typst::compile::<HtmlDocument>(world.as_ref());
    let html = output.and_then(|it| typst_html::html(&it)); // .map(|it| it.into_bytes());
    let result = ExtendedWarned {
        output: html.map_err(|it| it.resolve(world.as_ref())),
        warnings: warnings.resolve(world.as_ref()),
    };
    let _ = Box::into_raw(world); // Not to drop the world!
    JavaResult::pack(result)
}

#[no_mangle]
pub extern "C" fn compile_svg(
    world_ptr: *mut JavaWorld,
    from: i32,
    to: i32,
) -> JavaResult<ExtendedWarned<Result<Vec<String>, Vec<ExtendedSourceDiagnostic>>>> {
    compile_images(world_ptr, from, to, |page| typst_svg::svg(page))
}

#[no_mangle]
pub extern "C" fn compile_png(
    world_ptr: *mut JavaWorld,
    from: i32,
    to: i32,
    ppi: f32,
) -> JavaResult<ExtendedWarned<Result<Vec<Base16ByteArray>, Vec<ExtendedSourceDiagnostic>>>>
{
    compile_images(world_ptr, from, to, |page| {
        let pixmap = typst_render::render(page, ppi / 72.0);
        let buf = pixmap.encode_png().unwrap();
        Base16ByteArray(buf)
    })
}

fn compile_images<T: Serialize>(
    world_ptr: *mut JavaWorld,
    from: i32,
    to: i32,
    extractor: impl Fn(&Page) -> T,
) -> JavaResult<ExtendedWarned<Result<Vec<T>, Vec<ExtendedSourceDiagnostic>>>> {
    tick!();
    let world = unsafe { Box::from_raw(world_ptr) };
    tick!();
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world.as_ref());
    tick!();
    let pages = output.map(|document| {
        tick!();
        let mut doc_pages = document.pages;
        tick!();
        let start = (from as usize).min(doc_pages.len());
        tick!();
        let end = (to as usize).min(doc_pages.len());
        tick!();
        doc_pages
            .drain(start..end)
            .map(|it| extractor(&it))
            .collect::<Vec<_>>()
    });
    tick!();
    let result = ExtendedWarned {
        output: pages.map_err(|it| it.resolve(world.as_ref())),
        warnings: warnings.resolve(world.as_ref()),
    };
    tick!();
    let _ = Box::into_raw(world); // Not to drop the world!
    tick!();
    JavaResult::pack(result)
}

/// Convert [`chrono::DateTime`] to [`Datetime`]
fn convert_datetime<Tz: chrono::TimeZone>(
    date_time: chrono::DateTime<Tz>,
) -> Option<Datetime> {
    Datetime::from_ymd_hms(
        date_time.year(),
        date_time.month().try_into().ok()?,
        date_time.day().try_into().ok()?,
        date_time.hour().try_into().ok()?,
        date_time.minute().try_into().ok()?,
        date_time.second().try_into().ok()?,
    )
}
