use crate::memory_management::ThickBytePtr;
use std::mem;
use typst::syntax::{parse, parse_code, parse_math, SyntaxKind, SyntaxNode};

#[derive(Default)]
pub struct FlattenedSyntaxTree {
    pub marks: Vec<(SyntaxMark, i32)>,
    pub errors: Vec<u8>,
    pub errors_starts: Vec<i32>,
}

#[derive(Clone, Copy, Debug)]
pub enum SyntaxMark {
    NodeStart(SyntaxKind),
    NodeEnd,
    Error(i32),
}

impl SyntaxMark {
    fn encode(self) -> i32 {
        match self {
            SyntaxMark::NodeStart(SyntaxKind::End) => 0,
            SyntaxMark::NodeStart(SyntaxKind::Error) => 1,
            SyntaxMark::NodeStart(SyntaxKind::Shebang) => 2,
            SyntaxMark::NodeStart(SyntaxKind::LineComment) => 3,
            SyntaxMark::NodeStart(SyntaxKind::BlockComment) => 4,
            SyntaxMark::NodeStart(SyntaxKind::Markup) => 5,
            SyntaxMark::NodeStart(SyntaxKind::Text) => 6,
            SyntaxMark::NodeStart(SyntaxKind::Space) => 7,
            SyntaxMark::NodeStart(SyntaxKind::Linebreak) => 8,
            SyntaxMark::NodeStart(SyntaxKind::Parbreak) => 9,
            SyntaxMark::NodeStart(SyntaxKind::Escape) => 10,
            SyntaxMark::NodeStart(SyntaxKind::Shorthand) => 11,
            SyntaxMark::NodeStart(SyntaxKind::SmartQuote) => 12,
            SyntaxMark::NodeStart(SyntaxKind::Strong) => 13,
            SyntaxMark::NodeStart(SyntaxKind::Emph) => 14,
            SyntaxMark::NodeStart(SyntaxKind::Raw) => 15,
            SyntaxMark::NodeStart(SyntaxKind::RawLang) => 16,
            SyntaxMark::NodeStart(SyntaxKind::RawDelim) => 17,
            SyntaxMark::NodeStart(SyntaxKind::RawTrimmed) => 18,
            SyntaxMark::NodeStart(SyntaxKind::Link) => 19,
            SyntaxMark::NodeStart(SyntaxKind::Label) => 20,
            SyntaxMark::NodeStart(SyntaxKind::Ref) => 21,
            SyntaxMark::NodeStart(SyntaxKind::RefMarker) => 22,
            SyntaxMark::NodeStart(SyntaxKind::Heading) => 23,
            SyntaxMark::NodeStart(SyntaxKind::HeadingMarker) => 24,
            SyntaxMark::NodeStart(SyntaxKind::ListItem) => 25,
            SyntaxMark::NodeStart(SyntaxKind::ListMarker) => 26,
            SyntaxMark::NodeStart(SyntaxKind::EnumItem) => 27,
            SyntaxMark::NodeStart(SyntaxKind::EnumMarker) => 28,
            SyntaxMark::NodeStart(SyntaxKind::TermItem) => 29,
            SyntaxMark::NodeStart(SyntaxKind::TermMarker) => 30,
            SyntaxMark::NodeStart(SyntaxKind::Equation) => 31,
            SyntaxMark::NodeStart(SyntaxKind::Math) => 32,
            SyntaxMark::NodeStart(SyntaxKind::MathText) => 33,
            SyntaxMark::NodeStart(SyntaxKind::MathIdent) => 34,
            SyntaxMark::NodeStart(SyntaxKind::MathShorthand) => 35,
            SyntaxMark::NodeStart(SyntaxKind::MathAlignPoint) => 36,
            SyntaxMark::NodeStart(SyntaxKind::MathDelimited) => 37,
            SyntaxMark::NodeStart(SyntaxKind::MathAttach) => 38,
            SyntaxMark::NodeStart(SyntaxKind::MathPrimes) => 39,
            SyntaxMark::NodeStart(SyntaxKind::MathFrac) => 40,
            SyntaxMark::NodeStart(SyntaxKind::MathRoot) => 41,
            SyntaxMark::NodeStart(SyntaxKind::Hash) => 42,
            SyntaxMark::NodeStart(SyntaxKind::LeftBrace) => 43,
            SyntaxMark::NodeStart(SyntaxKind::RightBrace) => 44,
            SyntaxMark::NodeStart(SyntaxKind::LeftBracket) => 45,
            SyntaxMark::NodeStart(SyntaxKind::RightBracket) => 46,
            SyntaxMark::NodeStart(SyntaxKind::LeftParen) => 47,
            SyntaxMark::NodeStart(SyntaxKind::RightParen) => 48,
            SyntaxMark::NodeStart(SyntaxKind::Comma) => 49,
            SyntaxMark::NodeStart(SyntaxKind::Semicolon) => 50,
            SyntaxMark::NodeStart(SyntaxKind::Colon) => 51,
            SyntaxMark::NodeStart(SyntaxKind::Star) => 52,
            SyntaxMark::NodeStart(SyntaxKind::Underscore) => 53,
            SyntaxMark::NodeStart(SyntaxKind::Dollar) => 54,
            SyntaxMark::NodeStart(SyntaxKind::Plus) => 55,
            SyntaxMark::NodeStart(SyntaxKind::Minus) => 56,
            SyntaxMark::NodeStart(SyntaxKind::Slash) => 57,
            SyntaxMark::NodeStart(SyntaxKind::Hat) => 58,
            SyntaxMark::NodeStart(SyntaxKind::Prime) => 59,
            SyntaxMark::NodeStart(SyntaxKind::Dot) => 60,
            SyntaxMark::NodeStart(SyntaxKind::Eq) => 61,
            SyntaxMark::NodeStart(SyntaxKind::EqEq) => 62,
            SyntaxMark::NodeStart(SyntaxKind::ExclEq) => 63,
            SyntaxMark::NodeStart(SyntaxKind::Lt) => 64,
            SyntaxMark::NodeStart(SyntaxKind::LtEq) => 65,
            SyntaxMark::NodeStart(SyntaxKind::Gt) => 66,
            SyntaxMark::NodeStart(SyntaxKind::GtEq) => 67,
            SyntaxMark::NodeStart(SyntaxKind::PlusEq) => 68,
            SyntaxMark::NodeStart(SyntaxKind::HyphEq) => 69,
            SyntaxMark::NodeStart(SyntaxKind::StarEq) => 70,
            SyntaxMark::NodeStart(SyntaxKind::SlashEq) => 71,
            SyntaxMark::NodeStart(SyntaxKind::Dots) => 72,
            SyntaxMark::NodeStart(SyntaxKind::Arrow) => 73,
            SyntaxMark::NodeStart(SyntaxKind::Root) => 74,
            SyntaxMark::NodeStart(SyntaxKind::Not) => 75,
            SyntaxMark::NodeStart(SyntaxKind::And) => 76,
            SyntaxMark::NodeStart(SyntaxKind::Or) => 77,
            SyntaxMark::NodeStart(SyntaxKind::None) => 78,
            SyntaxMark::NodeStart(SyntaxKind::Auto) => 79,
            SyntaxMark::NodeStart(SyntaxKind::Let) => 80,
            SyntaxMark::NodeStart(SyntaxKind::Set) => 81,
            SyntaxMark::NodeStart(SyntaxKind::Show) => 82,
            SyntaxMark::NodeStart(SyntaxKind::Context) => 83,
            SyntaxMark::NodeStart(SyntaxKind::If) => 84,
            SyntaxMark::NodeStart(SyntaxKind::Else) => 85,
            SyntaxMark::NodeStart(SyntaxKind::For) => 86,
            SyntaxMark::NodeStart(SyntaxKind::In) => 87,
            SyntaxMark::NodeStart(SyntaxKind::While) => 88,
            SyntaxMark::NodeStart(SyntaxKind::Break) => 89,
            SyntaxMark::NodeStart(SyntaxKind::Continue) => 90,
            SyntaxMark::NodeStart(SyntaxKind::Return) => 91,
            SyntaxMark::NodeStart(SyntaxKind::Import) => 92,
            SyntaxMark::NodeStart(SyntaxKind::Include) => 93,
            SyntaxMark::NodeStart(SyntaxKind::As) => 94,
            SyntaxMark::NodeStart(SyntaxKind::Code) => 95,
            SyntaxMark::NodeStart(SyntaxKind::Ident) => 96,
            SyntaxMark::NodeStart(SyntaxKind::Bool) => 97,
            SyntaxMark::NodeStart(SyntaxKind::Int) => 98,
            SyntaxMark::NodeStart(SyntaxKind::Float) => 99,
            SyntaxMark::NodeStart(SyntaxKind::Numeric) => 100,
            SyntaxMark::NodeStart(SyntaxKind::Str) => 101,
            SyntaxMark::NodeStart(SyntaxKind::CodeBlock) => 102,
            SyntaxMark::NodeStart(SyntaxKind::ContentBlock) => 103,
            SyntaxMark::NodeStart(SyntaxKind::Parenthesized) => 104,
            SyntaxMark::NodeStart(SyntaxKind::Array) => 105,
            SyntaxMark::NodeStart(SyntaxKind::Dict) => 106,
            SyntaxMark::NodeStart(SyntaxKind::Named) => 107,
            SyntaxMark::NodeStart(SyntaxKind::Keyed) => 108,
            SyntaxMark::NodeStart(SyntaxKind::Unary) => 109,
            SyntaxMark::NodeStart(SyntaxKind::Binary) => 110,
            SyntaxMark::NodeStart(SyntaxKind::FieldAccess) => 111,
            SyntaxMark::NodeStart(SyntaxKind::FuncCall) => 112,
            SyntaxMark::NodeStart(SyntaxKind::Args) => 113,
            SyntaxMark::NodeStart(SyntaxKind::Spread) => 114,
            SyntaxMark::NodeStart(SyntaxKind::Closure) => 115,
            SyntaxMark::NodeStart(SyntaxKind::Params) => 116,
            SyntaxMark::NodeStart(SyntaxKind::LetBinding) => 117,
            SyntaxMark::NodeStart(SyntaxKind::SetRule) => 118,
            SyntaxMark::NodeStart(SyntaxKind::ShowRule) => 119,
            SyntaxMark::NodeStart(SyntaxKind::Contextual) => 120,
            SyntaxMark::NodeStart(SyntaxKind::Conditional) => 121,
            SyntaxMark::NodeStart(SyntaxKind::WhileLoop) => 122,
            SyntaxMark::NodeStart(SyntaxKind::ForLoop) => 123,
            SyntaxMark::NodeStart(SyntaxKind::ModuleImport) => 124,
            SyntaxMark::NodeStart(SyntaxKind::ImportItems) => 125,
            SyntaxMark::NodeStart(SyntaxKind::ImportItemPath) => 126,
            SyntaxMark::NodeStart(SyntaxKind::RenamedImportItem) => 127,
            SyntaxMark::NodeStart(SyntaxKind::ModuleInclude) => 128,
            SyntaxMark::NodeStart(SyntaxKind::LoopBreak) => 129,
            SyntaxMark::NodeStart(SyntaxKind::LoopContinue) => 130,
            SyntaxMark::NodeStart(SyntaxKind::FuncReturn) => 131,
            SyntaxMark::NodeStart(SyntaxKind::Destructuring) => 132,
            SyntaxMark::NodeStart(SyntaxKind::DestructAssignment) => 133,

            SyntaxMark::NodeEnd => 134,
            SyntaxMark::Error(idx) => 135 + idx,
        }
    }
}

pub fn flattened_tree(ast: SyntaxNode) -> FlattenedSyntaxTree {
    let mut tree = FlattenedSyntaxTree::default();
    flatten_into(&ast, &mut tree, 0, 0);
    tree
}

fn flatten_into(
    ast: &SyntaxNode,
    tree: &mut FlattenedSyntaxTree,
    idx: i32,
    depth: usize,
) {
    if ast.kind() == SyntaxKind::Error {
        tree.marks
            .push((SyntaxMark::Error(tree.errors_starts.len() as i32), idx));
        tree.errors_starts.push(tree.errors.len() as i32);
        let its_errors = ast.errors();
        let bytes = its_errors[0].message.as_bytes();
        tree.errors.extend(bytes);
        tree.marks.push((SyntaxMark::NodeEnd, idx + ast.len() as i32));
    } else {
        tree.marks.push((SyntaxMark::NodeStart(ast.kind()), idx));
        let children = ast.children();
        let mut tmp = idx;
        for child in children {
            flatten_into(child, tree, tmp, depth + 1);
            tmp += child.len() as i32;
        }
        tree.marks.push((SyntaxMark::NodeEnd, idx + ast.len() as i32));
    }
}

#[repr(C)]
pub struct CVec<T> {
    pub ptr: *mut T,
    pub len: i64,
    pub cap: i64,
}

impl<T> From<Vec<T>> for CVec<T> {
    fn from(value: Vec<T>) -> Self {
        let res = CVec {
            ptr: value.as_ptr() as *mut T,
            len: value.len() as i64,
            cap: value.capacity() as i64,
        };
        mem::forget(value);
        res
    }
}

impl<T> From<CVec<T>> for Vec<T> {
    fn from(value: CVec<T>) -> Self {
        unsafe { Vec::from_raw_parts(value.ptr, value.len as usize, value.cap as usize) }
    }
}

#[repr(C)]
pub struct CFlattenedSyntaxTree {
    pub marks: CVec<i64>,
    pub errors: CVec<u8>,
    pub errors_starts: CVec<i32>,
}

fn cfy(tree: FlattenedSyntaxTree) -> CFlattenedSyntaxTree {
    let marks: Vec<i64> = tree
        .marks
        .iter()
        .map(|it| ((it.0.encode() as i64) << 32) + it.1 as i64)
        .collect();
    CFlattenedSyntaxTree {
        marks: marks.into(),
        errors: tree.errors.into(),
        errors_starts: tree.errors_starts.into(),
    }
}

#[no_mangle]
pub extern "C" fn parse_syntax(string: ThickBytePtr, mode: i32) -> CFlattenedSyntaxTree {
    let input = string.to_str();
    let node = match mode {
        0 => parse(input.as_str()),      // Content
        1 => parse_code(input.as_str()), // Code
        2 => parse_math(input.as_str()), // Math
        _ => panic!("Unexpected mode {} for syntax", mode),
    };
    cfy(flattened_tree(node))
}

#[no_mangle]
pub extern "C" fn release_flattened_tree(tree: CFlattenedSyntaxTree) {
    let marks: Vec<i64> = tree.marks.into();
    let errors: Vec<u8> = tree.errors.into();
    let errors_starts: Vec<i32> = tree.errors_starts.into();
}
