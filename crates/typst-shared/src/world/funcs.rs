// Added by LDemetrios

use std::sync::{LazyLock, Mutex};

use typst_library::foundations::raw_string::RawString;
use crate::values::ToJson;
use serde::Deserialize;
use serde_json::json;
use typst::comemo::{Track, Tracked, TrackedMut};
use typst::syntax::SyntaxMode;
use typst::utils::Static;
use typst_eval::eval_string;
use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Args, Array, AutoValue, Bytes, CastInfo, Content, Context, Datetime, Decimal, Dict,
    Duration, Func, IntoValue, JvmObject, Label, Module, NativeFuncData, NativeFuncPtr,
    NativeParamInfo, NoneValue, Scope, Str, Styles, Symbol, Type, Value, Version,
};
use typst_library::layout::{Angle, Fr, Length, Ratio, Rel};
use typst_library::visualize::{Color, Gradient, Tiling};
use typst_library::World;

type HostNativeFuncSignature =
    dyn Fn(&mut Engine, Tracked<Context>, &mut Args) -> SourceResult<Value> + Send + Sync;
type ParamsFactory = dyn Fn() -> Vec<NativeParamInfo> + Send + Sync;
type ReturnsFactory = dyn Fn() -> CastInfo + Send + Sync;

unsafe extern "C" {
    pub fn invoke_native_function_by_ticket(
        result: &mut RawString,
        ticket: i64,
        args_len: u64,
        args_ptr: *mut u8,
        context_len: u64,
        context_ptr: *mut u8,
        session: i64
    );
}

#[derive(Debug, Deserialize)]
pub struct HostNativeFuncDescriptor {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub docs: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default = "default_true")]
    pub contextual: bool,
    #[serde(default)]
    pub params: Vec<HostNativeParamInfo>,
    #[serde(default = "default_cast_any")]
    pub returns: HostCastInfo,
}

#[derive(Debug, Deserialize)]
pub struct HostNativeParamInfo {
    pub name: String,
    #[serde(default)]
    pub docs: Option<String>,
    #[serde(default = "default_cast_any")]
    pub input: HostCastInfo,
    #[serde(default = "default_true")]
    pub positional: bool,
    #[serde(default)]
    pub named: bool,
    #[serde(default)]
    pub variadic: bool,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default)]
    pub settable: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HostCastInfo {
    pub kind: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub items: Vec<HostCastInfo>,
}

fn default_true() -> bool {
    true
}

fn default_cast_any() -> HostCastInfo {
    HostCastInfo { kind: "any".to_string(), name: None, items: vec![] }
}

pub struct HostNativeFuncData {
    pub data: Static<NativeFuncData>,
    data_raw: *mut NativeFuncData,
    function_raw: *mut HostNativeFuncSignature,
    params_factory_raw: *mut ParamsFactory,
    returns_factory_raw: *mut ReturnsFactory,
    owned_strings: Vec<*mut str>,
    owned_string_slices: Vec<*mut [&'static str]>,
    registered_names: Mutex<Vec<*mut str>>,
}

impl HostNativeFuncData {
    pub fn new(
        ticket: i64,
        descriptor: HostNativeFuncDescriptor,
    ) -> Result<Self, String> {
        let mut owned_strings = vec![];
        let mut owned_string_slices = vec![];

        let function_raw: *mut HostNativeFuncSignature =
            Box::into_raw(Box::new(move |engine, context, args| {
                let args_payload = serde_json::to_string(&Value::Args(args.clone()).to_json())
                    .expect("FATAL: host args serialization failed");
                let context_payload = serialize_context(context);
                // Host callbacks receive all call arguments serialized, so we must
                // mark them as consumed on the Typst side before returning.
                args.items.clear();
                let args_raw = RawString::from_string(args_payload);
                let context_raw = RawString::from_string(context_payload);
                let mut result = RawString::default();
                unsafe {
                    invoke_native_function_by_ticket(
                        &mut result,
                        ticket,
                        args_raw.len,
                        args_raw.ptr as *mut u8,
                        context_raw.len,
                        context_raw.ptr as *mut u8,
                        engine.world.session(),
                    );
                }
                args_raw.release();
                context_raw.release();
                let response = result.into_string().unwrap_or_else(|| {
                    "{\"Err\":\"Host callback returned invalid payload\"}".to_string()
                });
                let parsed: Result<String, String> = serde_json::from_str(&response)
                    .unwrap_or_else(|it| {
                        Err(format!("Host callback returned malformed result JSON: {it}"))
                    });
                match parsed {
                    Ok(code) => eval_string(
                        engine.routines,
                        engine.world,
                        TrackedMut::reborrow_mut(&mut engine.sink),
                        *engine
                            .introspector
                            .access("host native function callback result"),
                        context,
                        code.as_str(),
                        args.span,
                        SyntaxMode::Code,
                        Scope::new(),
                    ),
                    Err(message) => {
                        bail!(args.span, "{message}");
                    }
                }
            }));

        let name = leak_string(descriptor.name, &mut owned_strings);
        let title = leak_string(
            descriptor.title.unwrap_or_else(|| name.to_string()),
            &mut owned_strings,
        );
        let docs = leak_string(descriptor.docs.unwrap_or_default(), &mut owned_strings);
        let keywords_vec: Vec<&'static str> = descriptor
            .keywords
            .into_iter()
            .map(|it| leak_string(it, &mut owned_strings))
            .collect();
        let keywords = leak_str_slice(keywords_vec, &mut owned_string_slices);

        let params_static = convert_params(descriptor.params, &mut owned_strings)?;
        let returns_static = convert_cast_info(descriptor.returns, &mut owned_strings)?;

        let params_factory_raw: *mut ParamsFactory =
            Box::into_raw(Box::new(move || params_static.clone()));
        let returns_factory_raw: *mut ReturnsFactory =
            Box::into_raw(Box::new(move || returns_static.clone()));

        let data_raw = Box::into_raw(Box::new(NativeFuncData {
            function: NativeFuncPtr(unsafe { &*function_raw }),
            name,
            title,
            docs,
            keywords,
            contextual: descriptor.contextual,
            scope: LazyLock::new(&|| Scope::new()),
            params: LazyLock::new(unsafe { &*params_factory_raw }),
            returns: LazyLock::new(unsafe { &*returns_factory_raw }),
        }));

        Ok(Self {
            data: Static(unsafe { &*data_raw }),
            data_raw,
            function_raw,
            params_factory_raw,
            returns_factory_raw,
            owned_strings,
            owned_string_slices,
            registered_names: Mutex::new(vec![]),
        })
    }

    pub fn register_name(&self, value: String) -> &'static str {
        let raw = Box::into_raw(value.into_boxed_str());
        let mut names = self
            .registered_names
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        names.push(raw);
        unsafe { &*raw }
    }
}

impl Drop for HostNativeFuncData {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.data_raw));
            drop(Box::from_raw(self.function_raw));
            drop(Box::from_raw(self.params_factory_raw));
            drop(Box::from_raw(self.returns_factory_raw));
            for slice in self.owned_string_slices.drain(..) {
                drop(Box::from_raw(slice));
            }
            for string in self.owned_strings.drain(..) {
                drop(Box::from_raw(string));
            }
            let names = self
                .registered_names
                .get_mut()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for string in names.drain(..) {
                drop(Box::from_raw(string));
            }
        }
    }
}

fn leak_string(value: String, storage: &mut Vec<*mut str>) -> &'static str {
    let raw = Box::into_raw(value.into_boxed_str());
    storage.push(raw);
    unsafe { &*raw }
}

fn leak_str_slice(
    value: Vec<&'static str>,
    storage: &mut Vec<*mut [&'static str]>,
) -> &'static [&'static str] {
    let raw: *mut [&'static str] = Box::into_raw(value.into_boxed_slice());
    storage.push(raw);
    unsafe { &*raw }
}

fn convert_params(
    params: Vec<HostNativeParamInfo>,
    storage: &mut Vec<*mut str>,
) -> Result<Vec<NativeParamInfo>, String> {
    params
        .into_iter()
        .map(|it| {
            Ok(NativeParamInfo {
                name: leak_string(it.name, storage),
                docs: leak_string(it.docs.unwrap_or_default(), storage),
                input: convert_cast_info(it.input, storage)?,
                default: None,
                positional: it.positional,
                named: it.named,
                variadic: it.variadic,
                required: it.required,
                settable: it.settable,
            })
        })
        .collect()
}

fn convert_cast_info(
    info: HostCastInfo,
    storage: &mut Vec<*mut str>,
) -> Result<CastInfo, String> {
    match info.kind.as_str() {
        "any" => Ok(CastInfo::Any),
        "type" => {
            let name = info
                .name
                .ok_or_else(|| "CastInfo(type) requires `name`".to_string())?;
            let ty = type_by_name(&name)
                .ok_or_else(|| format!("Unknown type name in cast info: {name}"))?;
            Ok(CastInfo::Type(ty))
        }
        "union" => {
            let mut mapped = Vec::with_capacity(info.items.len());
            for item in info.items {
                mapped.push(convert_cast_info(item, storage)?);
            }
            Ok(CastInfo::Union(mapped))
        }
        other => Err(format!("Unsupported cast info kind: {other}")),
    }
}

fn type_by_name(name: &str) -> Option<Type> {
    let n = name.to_ascii_lowercase();
    Some(match n.as_str() {
        "none" => Type::of::<NoneValue>(),
        "auto" => Type::of::<AutoValue>(),
        "bool" | "boolean" => Type::of::<bool>(),
        "int" | "integer" => Type::of::<i64>(),
        "float" => Type::of::<f64>(),
        "length" => Type::of::<Length>(),
        "angle" => Type::of::<Angle>(),
        "ratio" => Type::of::<Ratio>(),
        "relative" => Type::of::<Rel<Length>>(),
        "fraction" | "fr" => Type::of::<Fr>(),
        "color" => Type::of::<Color>(),
        "gradient" => Type::of::<Gradient>(),
        "tiling" => Type::of::<Tiling>(),
        "symbol" => Type::of::<Symbol>(),
        "version" => Type::of::<Version>(),
        "str" | "string" => Type::of::<Str>(),
        "bytes" => Type::of::<Bytes>(),
        "label" => Type::of::<Label>(),
        "datetime" => Type::of::<Datetime>(),
        "decimal" => Type::of::<Decimal>(),
        "duration" => Type::of::<Duration>(),
        "content" => Type::of::<Content>(),
        "styles" => Type::of::<Styles>(),
        "array" => Type::of::<Array>(),
        "dict" | "dictionary" => Type::of::<Dict>(),
        "func" | "function" => Type::of::<Func>(),
        "args" | "arguments" => Type::of::<Args>(),
        "type" => Type::of::<Type>(),
        "module" => Type::of::<Module>(),
        "jvm-object" | "jvmobject" => Type::of::<JvmObject>(),
        _ => return None,
    })
}

fn serialize_context(context: Tracked<Context>) -> String {
    // let location = context.location().ok().to_json();
    // let styles = context.styles().ok().map(|it| it.to_map()).to_json();
    // let payload = json!({
    //     "type": "dict",
    //     "value": {
    //         "location": location,
    //         "styles": styles,
    //     }
    // });
    // serde_json::to_string(&payload).expect("FATAL: host context serialization failed")
    let payload = context.styles().ok().map(|it| it.to_map()).to_json();
    serde_json::to_string(&payload).expect("FATAL: host context serialization failed")
}
