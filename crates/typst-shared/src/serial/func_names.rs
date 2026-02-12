// Added by LDemetrios

use typst::utils::Static;
use typst_library::foundations::{Element, NativeFuncData};

pub fn elem_serial_name(elem: &Element) -> &'static str {
    match elem.title() {
        "Cases" => "math.cases",
        "Grid Cell" => "grid.cell",
        "Outline Entry" => "outline.entry",
        "Footnote Entry" => "footnote.entry",
        "Table Cell" => "table.cell",
        "Grid Header" => "grid.header",
        "Grid Footer" => "grid.footer",
        "Table Header" => "table.header",
        "Table Footer" => "table.footer",
        "Bullet List Item" => "list.item",
        "Numbered List Item" => "enum.item",
        "Term List Item" => "terms.item",
        "Table Horizontal Line" => "table.hline",
        "Table Vertical Line" => "table.vline",
        "Grid Horizontal Line" => "grid.hline",
        "Grid Vertical Line" => "grid.vline",
        "Curve Move" => "curve.move",
        "Curve Line" => "curve.line",
        "Curve Close" => "curve.close",
        "Curve Quadratic Segment" => "curve.quad",
        "Curve Cubic Segment" => "curve.cubic",
        "Paragraph Line" => "par.line",
        "Raw Text / Code Line" => "raw.line",
        "Elem" => "html.elem",
        _ => {
            let dollar = elem.docs().contains("$");
            match elem.name() {
                "elem" => "html.elem",
                "embed" => "pdf.embed",
                "frame" => "html.frame",
                "caption" => "figure.caption",
                "accent" => "math.accent",
                "binom" => "math.binom",
                "cancel" => "math.cancel",
                "cases" => "math.cases",
                "class" => "math.class",
                "class-class" => "math.class-class",
                "equation" => "math.equation",
                "display" => "math.display",
                "flush" => "place.flush",
                "frac" => "math.frac",
                "mat" => "math.mat",
                "align-point" => "math.align-point",
                "primes" => "math.primes",
                "stretch" => "math.stretch",
                "op" => "math.op",
                "vec" => "math.vec",
                "attach" => { if dollar { "math.attach" } else { "pdf.attach" } },
                "artifact" => "pdf.artifact",
                "scripts" => "math.scripts",
                "limits" => "math.limits",
                "lr" => "math.lr",
                "mid" => "math.mid",
                "root" => "math.root",
                "symbol" => "symbol-elem",
                "counter-update" => "counter.update",
                "state-update" => "state.update",
                "underline" => { if dollar { "math.underline" } else { "underline" } }
                "overline" => { if dollar { "math.overline" } else { "overline" } }
                "underbrace" => { if dollar { "math.underbrace" } else { "underbrace" } }
                "overbrace" => { if dollar { "math.overbrace" } else { "overbrace" } }
                "underbracket" => { if dollar { "math.underbracket" } else { "underbracket" } }
                "overbracket" => { if dollar { "math.overbracket" } else { "overbracket" } }
                "underparen" => { if dollar { "math.underparen" } else { "underparen" } }
                "overparen" => { if dollar { "math.overparen" } else { "overparen" } }
                "undershell" => { if dollar { "math.undershell" } else { "undershell" } }
                "overshell" => { if dollar { "math.overshell" } else { "overshell" } }
                _ => elem.name(),
            }
        }
    }
}

pub fn func_serial_name(func: &Static<NativeFuncData>) -> &'static str {
    match func.name {
        "display" => "math.display",
        "linear-rgb" => "color.linear-rgb",
        "hsl" => "color.hsl",
        "hsv" => "color.hsv",
        _ => match func.title {
            "Linear Gradient" => "gradient.linear",
            "Radial Gradient" => "gradient.radial",
            "Conic Gradient" => "gradient.conic",
            _ => func.name,
        },
    }
}
