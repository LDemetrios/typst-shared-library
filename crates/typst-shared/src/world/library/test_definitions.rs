// Added by LDemetrios

use std::io::Write;
use std::str::FromStr;

use typst::comemo::Tracked;
use typst::diag::{At, SourceResult, StrResult, bail};
use typst::engine::Engine;
use typst::foundations::{Array, Context, IntoValue, NoneValue, Repr, Value, func};
use typst::model::{Numbering, NumberingPattern};
use typst::syntax::Span;
use typst_library::Library;
use typst_library::visualize::Color;

// TODO These should be doubled by kotlin-based implementations.
// For testing of custom kotlin functions.

#[func]
pub fn test(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs != rhs {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
pub fn test_repr(lhs: Value, rhs: Value) -> StrResult<NoneValue> {
    if lhs.repr() != rhs.repr() {
        bail!("Assertion failed: {} != {}", lhs.repr(), rhs.repr());
    }
    Ok(NoneValue)
}

#[func]
pub fn print(#[variadic] values: Vec<Value>) -> NoneValue {
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
pub fn lines(
    engine: &mut Engine,
    context: Tracked<Context>,
    span: Span,
    count: u64,
    #[default(Numbering::Pattern(NumberingPattern::from_str("A").unwrap()))]
    numbering: Numbering,
) -> SourceResult<Value> {
    (1..=count)
        .map(|n| numbering.apply(engine, context, &[n]))
        .collect::<SourceResult<Array>>()?
        .join(Some('\n'.into_value()), None, None)
        .at(span)
}

pub fn conifer() -> Color {
    Color::from_u8(0x9f, 0xEB, 0x52, 0xFF)
}

pub fn forest() -> Color {
    Color::from_u8(0x43, 0xA1, 0x27, 0xFF)
}

pub fn register_test_definitions(lib: &mut Library) {
    lib.global.scope_mut().define_func::<test>();
    lib.global.scope_mut().define_func::<test_repr>();
    lib.global.scope_mut().define_func::<print>();
    lib.global.scope_mut().define_func::<lines>();
    lib.global.scope_mut().define("conifer", conifer());
    lib.global.scope_mut().define("forest", forest());
}
