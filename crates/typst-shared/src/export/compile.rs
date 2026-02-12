// Added by LDemetrios

use std::mem;
use crate::export::raw_bytes::Base64Bytes;
use crate::export::raw_string::RawString;
use crate::export::raw_time::RawNow;
use crate::export::utils::to_file_id;
use crate::export::world_parts::TicketedReader;
use crate::extended_info::{ExtendedSourceDiagnostic, ExtendedWarned, Resolve};
use crate::pdf_options::parse_pdf_options;
use crate::world::{CompositeWorld, FilesCache, FontCollection};
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use tiny_skia as sk;
use typst::syntax::Span;
use typst::utils::LazyHash;
use typst_html::HtmlDocument;
use typst_library::diag::{SourceDiagnostic, SourceResult, Warned};
use typst_library::foundations::{Base, Smart};
use typst_library::layout::{
    Abs, Frame, FrameItem, Page, PageRanges, PagedDocument, Transform,
};
use typst_library::visualize::Color;
use typst_library::{Library, World};
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards, Timestamp};
use crate::free_func;
use crate::world_parts::NoopWorld;

fn build_world<'a>(
    context: &'a mut FilesCache<TicketedReader>,
    fonts: &'a mut FontCollection,
    stdlib: &'a mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
) -> CompositeWorld<'a, TicketedReader> {
    let main = RawString::new(main_len, main_ptr);
    let now = RawNow::new(now_millis_or_flag, now_nanos);
    CompositeWorld::new(
        Some(context),
        Some(fonts),
        Some(stdlib),
        Some(to_file_id(main)),
        now.resolve(),
    )
}

pub fn diagnostics_from_message(
    message: impl Into<String>,
) -> Vec<ExtendedSourceDiagnostic> {
    let world = NoopWorld {};
    let diagnostic = SourceDiagnostic::error(Span::detached(), message.into());
    vec![diagnostic.resolve(&world)]
}

fn render_merged_with_links(document: &PagedDocument, pixel_per_pt: f32) -> sk::Pixmap {
    for page in &document.pages {
        let limit = Abs::cm(100.0);
        if page.frame.width() > limit || page.frame.height() > limit {
            panic!("overlarge frame: {:?}", page.frame.size());
        }
    }

    let gap = Abs::pt(1.0);
    let mut pixmap =
        typst_render::render_merged(document, pixel_per_pt, gap, Some(Color::BLACK));
    let gap = (pixel_per_pt * gap.to_pt() as f32).round();

    let mut y = 0.0;
    for page in &document.pages {
        let ts =
            sk::Transform::from_scale(pixel_per_pt, pixel_per_pt).post_translate(0.0, y);
        render_links(&mut pixmap, ts, &page.frame);
        y += (pixel_per_pt * page.frame.height().to_pt() as f32).round().max(1.0) + gap;
    }

    pixmap
}

fn render_links(canvas: &mut sk::Pixmap, ts: sk::Transform, frame: &Frame) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *item {
            FrameItem::Group(ref group) => {
                let ts = ts.pre_concat(to_sk_transform(&group.transform));
                render_links(canvas, ts, &group.frame);
            }
            FrameItem::Link(_, size) => {
                let w = size.x.to_pt() as f32;
                let h = size.y.to_pt() as f32;
                let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
                let mut paint = sk::Paint::default();
                paint.set_color_rgba8(40, 54, 99, 40);
                canvas.fill_rect(rect, &paint, ts, None);
            }
            _ => {}
        }
    }
}

fn to_sk_transform(transform: &Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = *transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_pt() as f32,
        ty.to_pt() as f32,
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn precompile_paged(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );

    let result_v: ExtendedWarned<Result<i64, Vec<ExtendedSourceDiagnostic>>> = typst::compile::<PagedDocument>(&world)
        .map(|it| it.map(|jt| Box::into_raw(Box::new(jt)) as usize as i64))
        .resolve(&world);

    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn compile_html(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );

    let Warned { output, warnings } = typst::compile::<HtmlDocument>(&world);
    let html = output.and_then(|it| typst_html::html(&it));

    let result_v: ExtendedWarned<Result<String, Vec<ExtendedSourceDiagnostic>>> =
        ExtendedWarned {
            output: html.map_err(|it| it.resolve(&world)),
            warnings: warnings.resolve(&world),
        };

    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn compile_svg(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
    from: i32,
    to: i32,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );

    let result_v: ExtendedWarned<Result<Vec<String>, Vec<ExtendedSourceDiagnostic>>> =
        compile_paged(&world, from, to, |page| typst_svg::svg(page));

    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn render_svg(
    result: &mut RawString,
    document: *mut PagedDocument,
    from: i32,
    to: i32,
) {
    let doc = unsafe { Box::from_raw(document) };

    let start = (from as usize).min(doc.pages.len());
    let end = (to as usize).min(doc.pages.len());

    let pages = doc.pages[start..end]
        .iter()
        .map(|it| typst_svg::svg(&it))
        .collect::<Vec<_>>();

    mem::forget(doc);

    RawString::write_from(result, &pages);
}

#[unsafe(no_mangle)]
pub extern "C" fn compile_png(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
    from: i32,
    to: i32,
    ppi: f64,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );

    let result_v: ExtendedWarned<
        Result<Vec<Base64Bytes>, Vec<ExtendedSourceDiagnostic>>,
    > = compile_paged(&world, from, to, |page| {
        let pixmap = typst_render::render(page, (ppi / 72.0) as f32);
        let buf = pixmap.encode_png().unwrap();
        Base64Bytes(buf)
    });

    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn render_png(
    result: &mut RawString,
    document: *mut PagedDocument,
    from: i32,
    to: i32,
    ppi: f64,
) {
    let doc = unsafe { Box::from_raw(document) };

    let start = (from as usize).min(doc.pages.len());
    let end = (to as usize).min(doc.pages.len());

    let pages: Vec<Base64Bytes> = doc.pages[start..end]
        .iter()
        .map(|it| {
            let pixmap = typst_render::render(it, (ppi / 72.0) as f32);
            let buf = pixmap.encode_png().unwrap();
            Base64Bytes(buf)
        })
        .collect::<Vec<_>>();
    mem::forget(doc);

    RawString::write_from(result, &pages);
}

#[unsafe(no_mangle)]
pub extern "C" fn compile_png_merged_with_links(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
    ppi: f64,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );

    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
    let rendered = output.map(|doc| {
        let pixmap = render_merged_with_links(&doc, (ppi / 72.0) as f32);
        Base64Bytes(pixmap.encode_png().unwrap())
    });

    let result_v: ExtendedWarned<Result<Base64Bytes, Vec<ExtendedSourceDiagnostic>>> =
        ExtendedWarned {
            output: rendered.map_err(|it| it.resolve(&world)),
            warnings: warnings.resolve(&world),
        };

    RawString::write_from(result, &result_v);
}

#[unsafe(no_mangle)]
pub extern "C" fn compile_pdf(
    result: &mut RawString,
    context: &mut FilesCache<TicketedReader>,
    fonts: &mut FontCollection,
    stdlib: &mut LazyHash<Library>,
    main_len: u64,
    main_ptr: *mut u8,
    now_millis_or_flag: i64,
    now_nanos: i32,
    options_len: u64,
    options_ptr: *mut u8,
) {
    let world = build_world(
        context,
        fonts,
        stdlib,
        main_len,
        main_ptr,
        now_millis_or_flag,
        now_nanos,
    );
    let raw = RawString::new(options_len, options_ptr);

    let options = match parse_pdf_options(raw) {
        Ok(value) => value,
        Err(value) => {
            let x: ExtendedWarned<Result<Base64Bytes, _>> =
                ExtendedWarned { output: Err(value), warnings: Vec::new() };
            result.write_from(&x);
            return;
        }
    };

    let Warned { output, warnings } = typst::compile::<PagedDocument>(&world);
    let pdf = output.and_then(|doc| typst_pdf::pdf(&doc, &options));

    let result_v: ExtendedWarned<Result<Base64Bytes, Vec<ExtendedSourceDiagnostic>>> =
        ExtendedWarned {
            output: pdf.map(|bytes| Base64Bytes(bytes)).map_err(|it| it.resolve(&world)),
            warnings: warnings.resolve(&world),
        };

    RawString::write_from(result, &result_v);
}


#[unsafe(no_mangle)]
pub extern "C" fn render_pdf(
    result: &mut RawString,
    document: *mut PagedDocument,
    context: &mut FilesCache<TicketedReader>,
    options_len: u64,
    options_ptr: *mut u8,
) {
    let doc = unsafe { Box::from_raw(document) };
    let raw = RawString::new(options_len, options_ptr);
    let options = match parse_pdf_options(raw) {
        Ok(value) => value,
        Err(value) => {
            let x: ExtendedWarned<Result<Base64Bytes, _>> =
                ExtendedWarned { output: Err(value), warnings: Vec::new() };
            result.write_from(&x);
            return;
        }
    };

    let world = CompositeWorld::new(
        Some(context), None, None, None, None,
    );

    let pdf = typst_pdf::pdf(&doc, &options)
        .map(|it| Base64Bytes(it))
        .resolve(&world);
    mem::forget(doc);


    RawString::write_from(result, &pdf);
}

free_func!(free_paged_document, PagedDocument);

pub fn compile_paged<T: Serialize>(
    world: &impl World,
    from: i32,
    to: i32,
    extractor: impl Fn(&Page) -> T,
) -> ExtendedWarned<Result<Vec<T>, Vec<ExtendedSourceDiagnostic>>> {
    let Warned { output, warnings } = typst::compile::<PagedDocument>(world);

    let pages = output.map(|document| {
        let mut doc_pages = document.pages;

        let start = (from as usize).min(doc_pages.len());

        let end = (to as usize).min(doc_pages.len());

        doc_pages
            .drain(start..end)
            .map(|it| extractor(&it))
            .collect::<Vec<_>>()
    });

    ExtendedWarned {
        output: pages.map_err(|it| it.resolve(world)),
        warnings: warnings.resolve(world),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn enforce_title(document: *mut PagedDocument, title_len: u64, title_ptr: *mut u8) {
    let mut doc = unsafe { Box::from_raw(document) };
    let title = RawString::new(title_len, title_ptr).into_string();
    doc.info.title = title.map(|it| it.into());
    mem::forget(doc);
}