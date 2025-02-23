use std::fmt::{self, Debug, Formatter};
use std::ops::{Add, Div, Mul, Neg};

use ecow::EcoString;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize};
use typst_utils::{Numeric, Scalar};

use crate::foundations::{repr, ty, Repr};

/// A ratio of a whole.
///
/// Written as a number, followed by a percent sign.
///
/// # Example
/// ```example
/// #set align(center)
/// #scale(x: 150%)[
///   Scaled apart.
/// ]
/// ```
#[ty(cast)]
#[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ratio(Scalar);

impl Ratio {
    /// A ratio of `0%` represented as `0.0`.
    pub const fn zero() -> Self {
        Self(Scalar::ZERO)
    }

    /// A ratio of `100%` represented as `1.0`.
    pub const fn one() -> Self {
        Self(Scalar::ONE)
    }

    /// Create a new ratio from a value, where `1.0` means `100%`.
    pub const fn new(ratio: f64) -> Self {
        Self(Scalar::new(ratio))
    }

    /// Get the underlying ratio.
    pub const fn get(self) -> f64 {
        (self.0).get()
    }

    /// Whether the ratio is zero.
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }

    /// Whether the ratio is one.
    pub fn is_one(self) -> bool {
        self.0 == 1.0
    }

    /// The absolute value of this ratio.
    pub fn abs(self) -> Self {
        Self::new(self.get().abs())
    }

    /// Return the ratio of the given `whole`.
    pub fn of<T: Numeric>(self, whole: T) -> T {
        let resolved = whole * self.get();
        if resolved.is_finite() {
            resolved
        } else {
            T::zero()
        }
    }
}

impl Debug for Ratio {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}%", self.get() * 100.0)
    }
}

impl Repr for Ratio {
    fn repr(&self) -> EcoString {
        repr::format_float_with_unit(self.get() * 100.0, "%")
    }
}

impl Neg for Ratio {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl Add for Ratio {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

typst_utils::sub_impl!(Ratio - Ratio -> Ratio);

impl Mul for Ratio {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0 * other.0)
    }
}

impl Mul<f64> for Ratio {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self(self.0 * other)
    }
}

impl Mul<Ratio> for f64 {
    type Output = Ratio;

    fn mul(self, other: Ratio) -> Ratio {
        other * self
    }
}

impl Div for Ratio {
    type Output = f64;

    fn div(self, other: Self) -> f64 {
        self.get() / other.get()
    }
}

impl Div<f64> for Ratio {
    type Output = Self;

    fn div(self, other: f64) -> Self {
        Self(self.0 / other)
    }
}

impl Div<Ratio> for f64 {
    type Output = Self;

    fn div(self, other: Ratio) -> Self {
        self / other.get()
    }
}

typst_utils::assign_impl!(Ratio += Ratio);
typst_utils::assign_impl!(Ratio -= Ratio);
typst_utils::assign_impl!(Ratio *= Ratio);
typst_utils::assign_impl!(Ratio *= f64);
typst_utils::assign_impl!(Ratio /= f64);


impl Serialize for Ratio {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map_ser = serializer.serialize_map(Some(2))?;
        map_ser.serialize_entry("type", "ratio")?;
        map_ser.serialize_entry("value", &self.get())?;
        map_ser.end()
    }
}

// No idea, just chatgpting
impl<'de> Deserialize<'de> for Ratio {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{MapAccess, Visitor};
        use std::fmt;

        struct RatioVisitor;

        impl<'de> Visitor<'de> for RatioVisitor {
            type Value = Ratio;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with keys 'type' and 'value'")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut ratio_type: Option<String> = None;
                let mut value: Option<f64> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "type" => {
                            ratio_type = Some(map.next_value()?);
                        }
                        "value" => {
                            value = Some(map.next_value()?);
                        }
                        _ => return Err(serde::de::Error::unknown_field(&key, &["type", "value"])),
                    }
                }

                if ratio_type.as_deref() != Some("ratio") {
                    return Err(serde::de::Error::custom("Expected type 'ratio'"));
                }

                let value = value.ok_or_else(|| serde::de::Error::missing_field("value"))?;
                Ok(Ratio::new(value))
            }
        }

        deserializer.deserialize_map(RatioVisitor)
    }
}
