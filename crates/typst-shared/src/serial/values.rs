// Added by LDemetrios

use crate::serial::func_names::{elem_serial_name, func_serial_name};
use base64::{Engine as _, engine::general_purpose};
use codex::ModifierSet;
use serde_json::Value::Object;
use serde_json::{Value as JsValue, json};
use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;
use std::sync::Arc;
use typst::ecow::EcoString;
use typst::syntax::{RootedPath, VirtualPath, VirtualRoot, ast};
use typst::utils::{LazyHash, Numeric, Scalar, Static};
use typst_library::foundations::List as SymList;
use typst_library::foundations::Variant as SymVariant;
use typst_library::foundations::{
    Arg, Args, Array, Bytes, ClosureNode, Content, Datetime, Decimal, Derived, Dict,
    Duration, Func, FuncInner, IntoValue, Label, Module, Plugin, Property, Regex, Scope,
    Selector, Smart, Str, Style, Styles, Symbol, SymbolInner, Transformation, Type,
    Value, Version,
};

use typst_library::introspection::{Counter, CounterKey, Location, State};
use typst_library::layout::{
    Abs, Alignment, Angle, Axes, Dir, Em, Fr, HAlignment, Length, Ratio, Rel, VAlignment,
};
use typst_library::visualize::{
    Color, ColorSpace, DashPattern, Gradient, LineCap, LineJoin, Paint, RelativeTo,
    Stroke, Tiling,
};

pub trait ToJson {
    fn to_json(&self) -> JsValue;
}

fn float_json(value: f64) -> JsValue {
    let value = if value.is_finite() {
        json!(value)
    } else if value.is_nan() {
        json!("nan")
    } else if value.is_sign_positive() {
        json!("inf")
    } else {
        json!("-inf")
    };
    json!({ "type" : "float", "value" : value })
}

macro_rules! auto_json {
    ($ty_constr:tt) => {
        impl ToJson for $ty_constr {
            fn to_json(&self) -> JsValue {
                self.clone().into_value().to_json()
            }
        }
    };
    ($ty_constr:tt < $($ty_arg:tt),* >) => {
        impl <$($ty_arg : ToJson + IntoValue + Copy,)+> ToJson for $ty_constr <$($ty_arg,)*> {
            fn to_json(&self) -> JsValue {
                self.clone().into_value().to_json()
            }
        }
    };
}

impl ToJson for Value {
    fn to_json(&self) -> JsValue {
        match self {
            Value::None => json!({"type" : "none"}),
            Value::Auto => json!({"type" : "auto"}),
            Value::Bool(b) => json!({"type" : "bool", "value" : b}),
            Value::Int(i) => json!({"type" : "int", "value" : i}),
            Value::Float(f) => float_json(*f),
            Value::Length(x) => x.to_json(),
            Value::Angle(x) => x.to_json(),
            Value::Ratio(x) => x.to_json(),
            Value::Relative(x) => x.to_json(),
            Value::Fraction(x) => x.to_json(),
            Value::Color(x) => x.to_json(),
            Value::Gradient(x) => x.to_json(),
            Value::Tiling(x) => x.to_json(),
            Value::Symbol(x) => x.to_json(),
            Value::Version(x) => x.to_json(),
            Value::Str(x) => x.to_json(),
            Value::Bytes(x) => x.to_json(),
            Value::Label(x) => x.to_json(),
            Value::Datetime(x) => x.to_json(),
            Value::Decimal(x) => x.to_json(),
            Value::Duration(x) => x.to_json(),
            Value::Content(x) => x.to_json(),
            Value::Styles(x) => x.to_json(),
            Value::Array(x) => x.to_json(),
            Value::Dict(x) => x.to_json(),
            Value::Func(x) => x.to_json(),
            Value::Args(x) => x.to_json(),
            Value::Type(x) => x.to_json(),
            Value::Module(x) => x.to_json(),
            Value::Dyn(x) => {
                if let Some(it) = x.downcast::<Stroke>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Dir>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Alignment>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<State>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Location>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Regex>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Counter>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<Selector>() {
                    it.to_json()
                } else if let Some(it) = x.downcast::<RootedPath>() {
                    it.to_json()
                } else {
                    panic!("{:?}", x.type_id())
                }
            }
        }
    }
}

impl ToJson for RootedPath {
    fn to_json(&self) -> JsValue {
        json!({
            "type": "path",
            "root": self.root().to_json(),
            "path": self.vpath().to_json(),
        })
    }
}

impl ToJson for VirtualRoot {
    fn to_json(&self) -> JsValue {
        match self {
            VirtualRoot::Project => json!({ "type" : "project" }),
            VirtualRoot::Package(p) => json!({
                "type" : "package",
                "namespace" : p.namespace,
                "name": p.name,
                "version" : {
                    "type" : "package-version",
                    "major" : p.version.major,
                    "minor" : p.version.minor,
                    "patch" : p.version.patch,
                }
            }),
        }
    }
}

impl ToJson for VirtualPath {
    fn to_json(&self) -> JsValue {
        json!(self.get_with_slash().to_string())
    }
}

impl ToJson for f64 {
    fn to_json(&self) -> JsValue {
        float_json(*self as f64)
    }
}

impl ToJson for i64 {
    fn to_json(&self) -> JsValue {
        json!({ "type" : "int", "value" : self })
    }
}

impl ToJson for bool {
    fn to_json(&self) -> JsValue {
        json!({ "type" : "bool", "value" : self })
    }
}

impl ToJson for Length {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "length",
            "abs" : self.abs.to_pt().to_json(),
            "em" : self.em.get().to_json(),
        })
    }
}

impl ToJson for Ratio {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "ratio",
            "value" : self.get().to_json(),
        })
    }
}

impl ToJson for Angle {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "angle",
            "value" : self.to_raw().to_json(),
        })
    }
}

impl ToJson for Fr {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "fraction",
            "value" : float_json(self.get() as f64),
        })
    }
}

impl ToJson for Rel<Length> {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "relative",
            "absolute-part":  float_json(self.abs.abs.to_pt()),
            "font-part": float_json(self.abs.em.get()),
            "ratio-part": float_json(self.rel.get()),
        })
    }
}

impl ToJson for Color {
    fn to_json(&self) -> JsValue {
        match self {
            Color::Luma(x) => json!({
                "type" : "luma",
                "lightness" : { "type" : "ratio", "value" : x.color.luma.to_json() },
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::Oklab(x) => json!({
                "type" : "oklab",
                "lightness" : { "type" : "ratio", "value" : x.color.l.to_json() },
                "a" : x.color.a.to_json() ,
                "b" : x.color.b.to_json() ,
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::Oklch(x) => json!({
                "type" : "oklch",
                "lightness" : { "type" : "ratio", "value" : x.color.l.to_json() },
                "chroma" : x.color.chroma.to_json() ,
                "hue" : Angle::deg(x.color.hue.into_degrees().into()).to_json(),
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::Rgb(x) => json!({
                "type" : "rgb",
                "red" : { "type" : "ratio", "value" : x.color.red.to_json() },
                "green" : { "type" : "ratio", "value" : x.color.green.to_json() },
                "blue" : { "type" : "ratio", "value" : x.color.blue.to_json() },
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::LinearRgb(x) => json!({
                "type" : "color.linear-rgb",
                "red" : { "type" : "ratio", "value" : x.color.red.to_json() },
                "green" : { "type" : "ratio", "value" : x.color.green.to_json() },
                "blue" : { "type" : "ratio", "value" : x.color.blue.to_json() },
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::Cmyk(x) => json!({
                "type" : "cmyk",
                "cyan" : { "type" : "ratio", "value" : x.c.to_json() },
                "magenta" : { "type" : "ratio", "value" : x.m.to_json() },
                "yellow" : { "type" : "ratio", "value" : x.y.to_json() },
                "key" : { "type" : "ratio", "value" : x.k.to_json() },
            }),
            Color::Hsl(x) => json!({
                "type" : "color.hsl",
                "hue" : Angle::deg(x.color.hue.into_degrees().into()).to_json(),
                "saturation" : { "type" : "ratio", "value" : x.color.saturation.to_json() },
                "lightness" : { "type" : "ratio", "value" : x.color.lightness.to_json() },
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
            Color::Hsv(x) => json!({
                "type" : "color.hsv",
                "hue" : Angle::deg(x.color.hue.into_degrees().into()).to_json(),
                "saturation" : { "type" : "ratio", "value" : x.color.saturation.to_json() },
                "value" : { "type" : "ratio", "value" : x.color.value.to_json() },
                "alpha" : { "type" : "ratio", "value" : x.alpha.to_json() },
            }),
        }
    }
}

fn stops_json(stops: &Vec<(Color, Ratio)>) -> JsValue {
    json!(
        {
            "type" : "array",
            "value" : JsValue::Array(
                stops.iter()
                    .map(|(c, r)| json!({"type" : "array", "value" : [c.to_json(), r.to_json()]}))
                    .collect::<Vec<_>>()
            )
        }
    )
}

impl ToJson for Gradient {
    fn to_json(&self) -> JsValue {
        match self {
            Gradient::Linear(x) => json!({
                "type" : "gradient.linear",
                "angle" : x.angle.to_json(),
                "space" : x.space.to_json(),
                "relative" : x.relative.to_json(),
                "stops" : stops_json(x.stops.as_ref()),
            }),
            Gradient::Radial(x) => json!({
                "type" : "gradient.radial",
                "center" : x.center.to_json(),
                "radius" : x.radius.to_json(),
                "focal-center" : x.focal_center.to_json(),
                "focal-radius" : x.focal_radius.to_json(),
                "space" : x.space.to_json(),
                "relative" : x.relative.to_json(),
                "stops" : stops_json(x.stops.as_ref()),
            }),
            Gradient::Conic(x) => json!({
                "type" : "gradient.conic",
                "angle" : x.angle.to_json(),
                "center" : x.center.to_json(),
                "space" : x.space.to_json(),
                "relative" : x.relative.to_json(),
                "stops" : stops_json(x.stops.as_ref()),
            }),
        }
    }
}

impl ToJson for Tiling {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "tiling",
            "size" : self.size().to_json(),
            "spacing" : self.spacing().to_json(),
            "relative" : self.relative().to_json(),
            "body" : self.body().to_json(),
        })
    }
}

impl ToJson for Symbol {
    fn to_json(&self) -> JsValue {
        // TODO In current state will require a lot of work on deserializer's site
        match &self.0 {
            SymbolInner::Single(x) => json!({
                "type" : "symbol.single",
                "text" : {
                    "type" : "str",
                    "value" : x,
                }
            }),
            SymbolInner::Complex(x) => json!({
                "type" : "symbol.complex",
                "value" : x.to_json()
            }),
            SymbolInner::Modified(x) => json!({
                "type" : "symbol.modified",
                "list" : {
                    "type":"array",
                    "value": match &x.list {
                        SymList::Static(y) => y.iter().map(|it| it.to_json()).collect::<Vec<JsValue>>(),
                        SymList::Runtime(y) => y.iter().map(|it| it.to_json()).collect::<Vec<JsValue>>(),
                    },
                },
                "modifiers" : x.modifiers.to_json(),
            }),
        }
    }
}

impl<S: Deref<Target = str> + serde::Serialize> ToJson for SymVariant<S> {
    fn to_json(&self) -> JsValue {
        json!({
            "type": "array",
            "value": [
                { "type": "str", "value": self.0.as_str() },
                { "type": "str", "value": self.1 }
            ]
        })
    }
}

impl<S: Deref<Target = str>> ToJson for ModifierSet<S> {
    fn to_json(&self) -> JsValue {
        json!(self.as_str())
    }
}

impl ToJson for Version {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "version",
            "value" : self.values().to_json(),
        })
    }
}

impl ToJson for Str {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "str",
            "value" : self.as_str(),
        })
    }
}

impl ToJson for Bytes {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "bytes",
            "value": general_purpose::STANDARD.encode(self.as_slice())
        })
    }
}

impl ToJson for Label {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "label",
            "value" : { "type" : "str", "value" : self.resolve().as_str() },
        })
    }
}

impl ToJson for Datetime {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "datetime",
            "year" : self.year().to_json(),
            "month" : self.month().to_json(),
            "day" : self.day().to_json(),
            "hour" : self.hour().to_json(),
            "minute" : self.minute().to_json(),
            "second" : self.second().to_json(),
        })
    }
}

impl ToJson for Decimal {
    // TODO Check if reversible
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "decimal",
            "value" : format!("{}", self.0),
        })
    }
}

impl ToJson for Duration {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "duration",
            "weeks" : self.weeks().to_json(),
            "days" : self.days().to_json(),
            "hours" : self.hours().to_json(),
            "minutes" : self.minutes().to_json(),
            "seconds" : self.seconds().to_json(),
        })
    }
}

impl ToJson for Content {
    fn to_json(&self) -> JsValue {
        let fields = self.fields();
        let elem = elem_serial_name(&self.elem());
        if elem == "counter.update" {
            let update = fields.get("update").unwrap();
            if let Value::Dict(dict) = update {
                let step = dict.get("step").unwrap();
                return json!({
                    "type": "counter.step",
                    "key": fields.get("key").unwrap().to_json(),
                    "level" : step.to_json(),
                });
            }
        }

        let mut fields = fields
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_json()))
            .collect::<Vec<(String, JsValue)>>();

        fields.push(("type".to_string(), json![elem_serial_name(&self.elem())]));
        Object(fields.into_iter().collect())
    }
}

impl ToJson for Styles {
    fn to_json(&self) -> JsValue {
        let mut out: Vec<JsValue> = Vec::new();
        let mut pending_size: Option<(usize, Property)> = None;
        let mut pending_cramped: Option<(usize, Property)> = None;

        for style in self.as_slice() {
            let style = style.deref();
            if let Style::Property(prop) = style {
                if is_math_equation_size(prop) {
                    if let Some((pos, prev_size)) = pending_size.take() {
                        out[pos] = Style::Property(prev_size).to_json();
                    }
                    if let Some((pos, cramped_prop)) = pending_cramped.take() {
                        if let Some(combined) =
                            combine_math_equation_styles(prop, &cramped_prop)
                        {
                            out[pos] = combined;
                        } else {
                            out[pos] = Style::Property(cramped_prop).to_json();
                            out.push(Style::Property(prop.clone()).to_json());
                        }
                    } else {
                        let pos = out.len();
                        out.push(JsValue::Null);
                        pending_size = Some((pos, prop.clone()));
                    }
                    continue;
                }
                if is_math_equation_cramped(prop) {
                    if let Some((pos, prev_cramped)) = pending_cramped.take() {
                        out[pos] = Style::Property(prev_cramped).to_json();
                    }
                    if let Some((pos, size_prop)) = pending_size.take() {
                        if let Some(combined) =
                            combine_math_equation_styles(&size_prop, prop)
                        {
                            out[pos] = combined;
                        } else {
                            out[pos] = Style::Property(size_prop).to_json();
                            out.push(Style::Property(prop.clone()).to_json());
                        }
                    } else {
                        let pos = out.len();
                        out.push(JsValue::Null);
                        pending_cramped = Some((pos, prop.clone()));
                    }
                    continue;
                }
            }

            out.push(style.to_json());
        }

        if let Some((pos, prop)) = pending_size.take() {
            out[pos] = Style::Property(prop).to_json();
        }
        if let Some((pos, prop)) = pending_cramped.take() {
            out[pos] = Style::Property(prop).to_json();
        }

        json!({
            "type" : "styles",
            "value": { "type" : "array", "value" : out },
        })
    }
}

fn is_math_equation_size(prop: &Property) -> bool {
    elem_serial_name(&prop.elem) == "math.equation" && prop.id == 6
}

fn is_math_equation_cramped(prop: &Property) -> bool {
    elem_serial_name(&prop.elem) == "math.equation" && prop.id == 8
}

fn math_size_func_name(value: &Value) -> Option<&'static str> {
    let Value::Str(s) = value else {
        return None;
    };
    let normalized = s
        .as_str()
        .to_lowercase()
        .chars()
        .filter(|c| *c != '-' && *c != '_' && *c != ' ')
        .collect::<String>();
    match normalized.as_str() {
        "display" => Some("math.display"),
        "text" | "inline" => Some("math.inline"),
        "script" => Some("math.script"),
        "scriptscript" | "sscript" => Some("math.sscript"),
        _ => None,
    }
}

fn combine_math_equation_styles(
    size_prop: &Property,
    cramped_prop: &Property,
) -> Option<JsValue> {
    if !is_math_equation_size(size_prop) || !is_math_equation_cramped(cramped_prop) {
        return None;
    }

    let size_value = size_prop.value.0.to_value();
    let cramped_value = cramped_prop.value.0.to_value();
    let func_name = math_size_func_name(&size_value)?;
    let Value::Bool(cramped) = cramped_value else {
        return None;
    };

    let func = json!({
        "type": "native-function",
        "name": func_name,
        "ptr": 0,
    });

    let args = json!({
        "type" : "args",
        "positional" : { "type" : "array", "value": [] },
        "named" : { "type" : "dict", "value": { "cramped": Value::Bool(cramped).to_json() } },
    });

    let transform = json!({
        "type" : "func.with",
        "func" : func,
        "args" : args,
    });

    let outside = size_prop.outside || cramped_prop.outside;

    Some(json!({
        "type" : "show-rule",
        "transform" : transform,
        "outside" : outside,
    }))
}

impl ToJson for Style {
    fn to_json(&self) -> JsValue {
        match self {
            Style::Property(x) => {
                let value = format!("{:?}", x.value.0);
                let orig = match &x.origin {
                    Some(x) => x.to_json(),
                    None => json!(null),
                };
                let reconstructed = match &x.origin {
                    Some(_) => json!(null),
                    None => x.value.0.to_value().to_json(),
                };
                json!({
                    "type" : "set-rule",
                    "elem" : elem_serial_name(&x.elem),
                    "id" : x.elem.field_name(x.id),
                    "value" : value,
                    "original" : orig,
                    "reconstructed" : reconstructed,
                    "internals" : {
                        "type" : "internals",
                        "id" : x.id,
                        "span": x.span.into_raw(),
                        "liftable": x.liftable,
                        "outside": x.outside,
                    }
                })
            }
            Style::Recipe(x) => {
                let selector = x.selector();
                if let Some(selector) = selector {
                    json!({
                        "type" : "show-rule",
                        "selector" : selector.to_json(),
                        "transform" : x.transform().to_json(),
                        "outside" : x.outside,
                    })
                } else {
                    json!({
                        "type" : "show-rule",
                        "transform" : x.transform().to_json(),
                        "outside" : x.outside,
                    })
                }
            }
            Style::Revocation(i) => json!({ // TODO What is this
                "type" : "revocation",
                "value" : i.0
            }),
        }
    }
}

impl ToJson for Transformation {
    fn to_json(&self) -> JsValue {
        match self {
            Transformation::Content(x) => x.to_json(),
            Transformation::Func(x) => x.to_json(),
            Transformation::Style(x) => x.to_json(),
        }
    }
}

impl ToJson for Array {
    fn to_json(&self) -> JsValue {
        self.as_slice().to_json()
    }
}

impl ToJson for Dict {
    fn to_json(&self) -> JsValue {
        let v = self
            .iter()
            .map(|(k, v)| (k, v.to_json()))
            .collect::<Vec<_>>()
            .into_iter()
            .collect::<BTreeMap<_, _>>();

        json!({
            "type" : "dict",
            "value" : v
        })
    }
}

impl ToJson for Func {
    fn to_json(&self) -> JsValue {
        match &self.inner {
            FuncInner::Native(f) => json!({
                "type" : "native-function",
                "name" : func_serial_name(f),
                "ptr" : to_ptr(f),
            }),
            FuncInner::Element(e) => json!({
                "type" : "element",
                "name" : elem_serial_name(e),
                "ptr" : to_ptr(&e.0)
            }),
            FuncInner::Closure(c) => json!({
                "type" : "closure",
                "node" : c.node.to_json(),
                "defaults" : c.defaults.to_json(),
                "captured" : c.captured.to_json(),
                "num-pos-params" : c.num_pos_params,
            }),
            FuncInner::Plugin(p) => json!({
                "type" : "plugin-function",
                "plugin" : p.plugin.to_json(),
                "name" : p.name(),
            }),
            FuncInner::With(w) => json!({
                "type" : "func.with",
                "func" : w.0.to_json(),
                "args" : w.1.to_json(),
            }),
        }
    }
}

fn to_ptr<T: 'static>(data: &Static<T>) -> usize {
    let x: &'static T = data.0;
    x as *const _ as _
}

impl ToJson for ClosureNode {
    fn to_json(&self) -> JsValue {
        match self {
            ClosureNode::Closure(x) => {
                let name = x
                    .cast::<ast::Closure>()
                    .and_then(|closure| closure.name().map(|id| id.get().to_string()));
                let mut obj = serde_json::Map::new();
                obj.insert("type".to_string(), json!("real-closure"));
                obj.insert("value".to_string(), json!(x.to_text_ref()));
                if let Some(name) = name {
                    obj.insert("name".to_string(), json!(name));
                }
                Object(obj)
            }
            ClosureNode::Context(x) => {
                json!({"type" : "synt-closure", "value" : x.to_text_ref() })
            }
        }
    }
}

impl ToJson for Scope {
    fn to_json(&self) -> JsValue {
        json!(
            {
                "type" : "dict",
                "value" : Object(
                    self
                        .clone()
                        .iter()
                        .collect::<Vec<_>>()
                        .into_iter()
                        .map(|(k, v)| (k.into(), v.read().to_json()))
                        .collect(),
                ),
            }
        )
    }
}

impl ToJson for Args {
    fn to_json(&self) -> JsValue {
        let mut positional = Vec::new();
        let mut named = BTreeMap::new();
        for item in &self.items {
            if let Some(name) = &item.name {
                named.insert(name.to_string(), item.value.v.to_json());
            } else {
                positional.push(item.value.v.to_json());
            }
        }
        json!({
            "type" : "args",
            "positional" : { "type" : "array", "value": positional },
            "named" : { "type" : "dict", "value": named},
        })
    }
}

impl ToJson for Arg {
    fn to_json(&self) -> JsValue {
        json!({
            "name" : self.name.to_json(),
            "value" : self.value.v.to_json(),
        })
    }
}

impl ToJson for Type {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "type",
            "name" : { "type": "str", "value" : self.short_name() },
        })
    }
}

impl ToJson for Module {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "module",
            "name" : { "type": "str", "value": self.name().to_json() },
            "content" : self.clone().content().to_json(),
            // "scope" : self.scope().to_json(),
            "scope" : { "type" : "dict", "value": {} },
        })
    }
}

impl ToJson for Arc<Plugin> {
    fn to_json(&self) -> JsValue {
        todo!()
        // TODO should we keep this as pointer, or incorporate all intrinsics?
        // The latter is probably impossible... The former requires deeper investigation about memory safety.
    }
}

impl ToJson for Selector {
    fn to_json(&self) -> JsValue {
        match self {
            Selector::Elem(elem, with) => {
                let toj: Option<HashMap<&str, JsValue>> = with.as_ref().map(|w| {
                    w.iter()
                        .map(|it| {
                            let key = elem.field_name(it.0).unwrap();
                            let value = it.1.to_json();
                            (key, value)
                        })
                        .into_iter()
                        .collect::<HashMap<_, _>>()
                });
                match toj {
                    Some(x) if x.is_empty() => json!({
                        "type" : "element",
                        "name" : elem_serial_name(elem),
                        "ptr" : to_ptr(&elem.0),
                    }),
                    None => json!({
                        "type" : "element",
                        "name" : elem_serial_name(elem),
                        "ptr" : to_ptr(&elem.0),
                    }),
                    Some(x) => json!({
                        "type" : "selector.where",
                        "func" : {
                            "type" : "element",
                            "name" : elem_serial_name(elem),
                            "ptr" : to_ptr(&elem.0),
                        },
                        "args" : {
                            "type" : "args",
                            "named" : { "type" : "dict", "value" : x },
                            "positional" : {"type": "array", "value": []}
                        },
                    }),
                }
            }
            Selector::Location(it) => it.to_json(),
            Selector::Label(it) => it.to_json(),
            Selector::Regex(it) => it.to_json(),
            Selector::Can(it) => todo!(),
            Selector::Or(it) => json!({
                "type" : "selector.or",
                "options" : it.to_json()
            }),
            Selector::And(it) => json!({
                "type" : "selector.and",
                "options" : it.to_json()
            }),
            Selector::Before { selector, end, inclusive } => json!({
                "type" : "selector.before",
                "selector" : selector.to_json(),
                "end" : end.to_json(),
                "inclusive" : inclusive,
            }),
            Selector::After { selector, start, inclusive } => json!({
                "type" : "selector.after",
                "selector" : selector.to_json(),
                "start" : start.to_json(),
                "inclusive" : inclusive,
            }),
        }
    }
}

impl ToJson for Stroke {
    fn to_json(&self) -> JsValue {
        json!({
            "type": "stroke",
            "paint" : self.paint.to_json(),
            "thickness" : self.thickness.to_json(),
            "cap" : self.cap.to_json(),
            "join" : self.join.to_json(),
            "dash" : self.dash.to_json(),
            "miter-limit" : self.miter_limit.map(|it| it.scalar()).to_json(),
        })
    }
}

auto_json!(LineCap);
auto_json!(LineJoin);
auto_json!(DashPattern);

impl ToJson for Scalar {
    fn to_json(&self) -> JsValue {
        float_json(self.0)
    }
}

impl ToJson for Dir {
    fn to_json(&self) -> JsValue {
        match self {
            Dir::LTR => json!({ "type": "direction", "value" : "ltr" }),
            Dir::RTL => json!({ "type": "direction", "value" : "rtl" }),
            Dir::TTB => json!({ "type": "direction", "value" : "ttb" }),
            Dir::BTT => json!({ "type": "direction", "value" : "btt" }),
        }
    }
}

impl ToJson for Alignment {
    fn to_json(&self) -> JsValue {
        match self {
            Alignment::H(h) => json!({
                "type": "h-alignment",
                "value": h.to_json(),
            }),
            Alignment::V(v) => json!({
                "type": "v-alignment",
                "value": v.to_json(),
            }),
            Alignment::Both(h, v) => json!({
                "type": "alignment",
                "h": {
                    "type": "h-alignment",
                    "value": h.to_json(),
                },
                "v": {
                    "type": "v-alignment",
                    "value": v.to_json(),
                },
            }),
        }
    }
}

impl ToJson for HAlignment {
    fn to_json(&self) -> JsValue {
        match self {
            HAlignment::Left => json!("left"),
            HAlignment::Right => json!("right"),
            HAlignment::Center => json!("center"),
            HAlignment::Start => json!("start"),
            HAlignment::End => json!("end"),
        }
    }
}

impl ToJson for VAlignment {
    fn to_json(&self) -> JsValue {
        match self {
            VAlignment::Top => json!("top"),
            VAlignment::Bottom => json!("bottom"),
            VAlignment::Horizon => json!("horizon"),
        }
    }
}

impl ToJson for State {
    fn to_json(&self) -> JsValue {
        json!({
            "type": "state",
            "key" : self.key().to_json(),
            "init" : self.init().to_json(),
        })
    }
}

impl ToJson for Location {
    fn to_json(&self) -> JsValue {
        json!(self.hash().to_string())
    }
}

impl ToJson for Regex {
    fn to_json(&self) -> JsValue {
        json!({
            "type" : "regex",
            "regex" : { "type" : "str", "value" : self.pattern() },
        })
    }
}

impl ToJson for Counter {
    fn to_json(&self) -> JsValue {
        let value = match &self.0 {
            CounterKey::Page => json!({
                "type" : "counter.page",
            }),
            CounterKey::Selector(it) => it.to_json(),
            CounterKey::Str(it) => it.to_json(),
        };
        json!({
            "type" : "counter",
            "key" : value,
        })
    }
}

impl ToJson for RelativeTo {
    fn to_json(&self) -> JsValue {
        let x = match self {
            RelativeTo::Self_ => "self",
            RelativeTo::Parent => "parent",
        };
        json!({
            "type" : "str",
            "value" : x,
        })
    }
}

impl ToJson for Abs {
    fn to_json(&self) -> JsValue {
        Length { abs: *self, em: Em::zero() }.to_json()
    }
}

impl<S: ToJson, D> ToJson for Derived<S, D> {
    fn to_json(&self) -> JsValue {
        self.source.to_json()
    }
}

impl<T: ToJson> ToJson for Smart<T> {
    fn to_json(&self) -> JsValue {
        match self {
            Smart::Auto => json!({"type" : "auto"}),
            Smart::Custom(x) => x.to_json(),
        }
    }
}

impl<T: ToJson> ToJson for LazyHash<T> {
    fn to_json(&self) -> JsValue {
        self.deref().to_json()
    }
}

///////////////////////////////// Rust's native types /////////////////////////////////

// TODO ?! Check, how many auto-inferences are disrupted upon deletion

macro_rules! numeric {
    ($tp:ty, $repr:expr) => {
        impl ToJson for $tp {
            fn to_json(&self) -> JsValue {
                if $repr == "float" {
                    float_json(*self as f64)
                } else {
                    json!({ "type" : $repr , "value": self })
                }
            }
        }
    }
}

numeric!(u8, "int");
numeric!(u16, "int");
numeric!(u32, "int");
numeric!(u64, "int");
numeric!(i8, "int");
numeric!(i16, "int");
numeric!(i32, "int");
// numeric!(i64, "int");

numeric!(f32, "float");
// numeric!(f64, "float");

impl<T: ToJson> ToJson for [T] {
    fn to_json(&self) -> JsValue {
        json!({ "type": "array", "value": JsValue::Array(self.iter().map(|x| x.to_json()).collect()) })
    }
}

impl<T: ToJson> ToJson for Option<T> {
    fn to_json(&self) -> JsValue {
        match self {
            Some(x) => x.to_json(),
            None => json!({ "type": "none" }),
        }
    }
}

impl ToJson for EcoString {
    fn to_json(&self) -> JsValue {
        json!(self)
    }
}

impl<T: ?Sized + ToJson> ToJson for &T {
    fn to_json(&self) -> JsValue {
        (*self).to_json()
    }
}

//////////////////////////////////// Via IntoValue ////////////////////////////////////

// Only explicit to_json via into_value is allowed, for purposes of tracking such types.

auto_json!(Axes<T>); // arr [x, y]
auto_json!(Vec<T>); // arr
auto_json!(ColorSpace); // func
auto_json!(Paint);
