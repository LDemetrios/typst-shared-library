// Added by LDemetrios

use crate::foundations::raw_string::RawString;
use crate::foundations::{
    CastInfo, Func, Label, LocatableSelector, Reflect, Repr, Selector, Str, Type, Value,
};
use crate::introspection::Location;
use ecow::{EcoString, eco_format};
use std::hash::{Hash, Hasher};
use typst_library::diag::HintedStrResult;
use typst_library::foundations::{FromValue, IntoValue};
use typst_macros::{cast, func, scope, ty};
use wasmi::Val;

unsafe extern "C" {
    pub fn jvm_object_to_string(result: &mut RawString, session: i64, id: i64);
    pub fn jvm_object_get_class(result: &mut RawString, session: i64, id: i64);
    pub fn jvm_object_hash_code(session: i64, id: i64) -> i32;

    pub fn jvm_object_equals(
        session_left: i64,
        id_left: i64,
        session_right: i64,
        id_right: i64,
    ) -> i32;
}

#[ty(scope, cast)]
#[derive(Debug, Default, Copy, Clone, Eq)]
pub struct JvmObject {
    pub session: i64,
    pub id: i64,
}

#[scope]
impl JvmObject {
    #[func(constructor)]
    pub fn construct(session: i64, id: i64) -> HintedStrResult<JvmObject> {
        Ok(JvmObject { session, id })
    }

    #[func]
    pub fn to_string(&self) -> HintedStrResult<Str> {
        let result = unsafe {
            let mut res = RawString::default();
            jvm_object_to_string(&mut res, self.session, self.id);
            res.into_string().unwrap()
        };
        Ok(Str::from(result))
    }

    #[func]
    pub fn get_class(&self) -> HintedStrResult<Str> {
        let result = unsafe {
            let mut res = RawString::default();
            jvm_object_get_class(&mut res, self.session, self.id);
            res.into_string().unwrap()
        };
        Ok(Str::from(result))
    }
}

impl Repr for JvmObject {
    fn repr(&self) -> EcoString {
        eco_format!(
            "jvm-object({}, {}, {})",
            self.session,
            self.id,
            self.to_string().unwrap_or(Str::from("<Can't toString>")).repr()
        )
    }
}

impl Reflect for JvmObject {
    fn input() -> CastInfo {
        CastInfo::Type(Type::of::<Self>())
    }

    fn output() -> CastInfo {
        CastInfo::Type(Type::of::<Self>())
    }

    fn castable(value: &Value) -> bool {
        matches!(value, Value::JvmObject(_))
    }
}

impl IntoValue for JvmObject {
    fn into_value(self) -> Value {
        Value::JvmObject(self)
    }
}

impl FromValue for JvmObject {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        match value {
            Value::JvmObject(v) => Ok(v),
            _ => Err(<Self as Reflect>::error(&value)),
        }
    }
}

impl Hash for JvmObject {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(unsafe { jvm_object_hash_code(self.session, self.id) })
    }
}

impl PartialEq for JvmObject {
    fn eq(&self, rhs: &Self) -> bool {
        unsafe { jvm_object_equals(self.session, self.id, rhs.session, rhs.id) == 1 }
    }
}
