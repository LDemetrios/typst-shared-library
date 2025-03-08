use crate::extended_info::{ExtendedFileDescriptor, ExtendedFileResult};
use crate::java_world::JavaWorld;
use crate::memory_management::{Base16ByteArray, JavaResult, ThickBytePtr};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::io::Write;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::str::FromStr;
use typst::comemo::{Track, Tracked};
use typst::syntax::Span;
use typst::utils::{tick, LazyHash, SmallBitSet};
use typst_eval::eval_string;
use typst_kit::fonts::Fonts;
use typst_library::diag::{bail, At, SourceResult, StrResult};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Array, Context, Dict, IntoValue, NoneValue, Repr, Scope, Str, Value,
};
use typst_library::model::{Numbering, NumberingPattern};
use typst_library::routines::EvalMode;
use typst_library::visualize::Color;
use typst_library::{Features, Library, World};
use typst_macros::func;

fn eval_with_world(string: &str, world: &dyn World) -> Value {
    eval_string(
        &typst::ROUTINES,
        world.track(),
        string,
        Span::detached(),
        EvalMode::Code,
        Scope::default(),
    )
    .unwrap()
}

extern "C" fn main_noop() -> JavaResult<ExtendedFileDescriptor> {
    panic!();
}
extern "C" fn file_noop(
    _: ThickBytePtr,
) -> JavaResult<ExtendedFileResult<Base16ByteArray>> {
    panic!();
}

fn eval_no_world(string: &str) -> Value {
    let fonts = Fonts::searcher()
        .include_system_fonts(true)
        .search_with(&(vec![] as Vec<PathBuf>));

    let library = {
        let inputs: Dict = Dict::new();

        let features = vec![typst::Feature::Html].into_iter().collect();

        Library::builder().with_inputs(inputs).with_features(features).build()
    };

    let java_world = JavaWorld {
        library: LazyHash::new(library),
        book: LazyHash::new(fonts.book),
        main_callback: main_noop,
        file_callback: file_noop,
        fonts: fonts.fonts,
        files: Mutex::new(HashMap::new()),
        now: None,
        package_storage: None,
        auto_load_central: false,
    };

    eval_with_world(string, &java_world)
}

#[no_mangle]
pub extern "C" fn create_stdlib(
    features: c_int,
    inputs_thick: ThickBytePtr,
) -> *mut Library {
    tick!("{:?}", features);
    let inputs_str = inputs_thick.to_str();

    let inputs = eval_no_world(inputs_str.as_str()).cast::<Dict>().unwrap();

    let mut features_bitset = SmallBitSet::default();
    tick!();
    for i in 0..1 {
        if features >> i & 1 == 1 {
            features_bitset.insert(i as usize)
        }
    }
    tick!("{:?}", features_bitset);
    tick!("{:?}", Features(features_bitset.clone()));

    let mut lib = Library::builder()
        .with_inputs(inputs)
        .with_features(Features(features_bitset))
        .build();

    // Temporary, for testing purposes.

    lib.global.scope_mut().define_func::<test>();
    lib.global.scope_mut().define_func::<test_repr>();
    lib.global.scope_mut().define_func::<print>();
    lib.global.scope_mut().define_func::<lines>();
    lib.global
        .scope_mut()
        .define("conifer", Color::from_u8(0x9f, 0xEB, 0x52, 0xFF));
    lib.global
        .scope_mut()
        .define("forest", Color::from_u8(0x43, 0xA1, 0x27, 0xFF));

    tick!();

    Box::into_raw(Box::new(lib))
}

#[func]
fn test(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs != rhs {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
fn test_repr(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs.repr() != rhs.repr() {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
fn print(#[variadic] values: Vec<Value>) -> NoneValue {
    let mut out = std::io::stdout().lock();
    write!(out, "> ").unwrap();
    for (i, value) in values.into_iter().enumerate() {
        if i > 0 {
            write!(out, ", ").unwrap();
        }
        write!(out, "{value:?}").unwrap();
    }
    writeln!(out).unwrap();
    NoneValue
}

/// Generates `count` lines of text based on the numbering.
#[func]
fn lines(
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    count: usize,
    #[default(Numbering::Pattern(NumberingPattern::from_str("A").unwrap()))]
    numbering: Numbering,
) -> SourceResult<Value> {
    (1..=count)
        .map(|n| numbering.apply(engine, context, &[n]))
        .collect::<SourceResult<Array>>()?
        .join(Some('\n'.into_value()), None)
        .at(span)
}
