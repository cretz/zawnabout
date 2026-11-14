//! Named, ranged tunables. Every tunable struct is declared with [`params!`] so debug
//! tooling (sliders, `set`/`get` commands, scripts) can reach every field by name.

/// One tunable as debug tooling sees it.
#[derive(Clone, Copy, Debug)]
pub struct ParamDef {
    /// Field name without the group prefix.
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    /// True when the value is a toggle (shown as a checkbox, stored as 0 or 1).
    pub toggle: bool,
    pub doc: &'static str,
}

/// A struct whose fields are tunables.
pub trait Params: Default {
    /// Group prefix used in full names, such as `flight` in `flight.gravity`.
    const GROUP: &'static str;
    fn defs() -> &'static [ParamDef];
    fn get(&self, name: &str) -> Option<f32>;
    /// Returns false for an unknown name.
    fn set(&mut self, name: &str, value: f32) -> bool;
}

/// A field type a tunable can have.
pub trait ParamValue: Copy {
    const TOGGLE: bool;
    fn to_f32(self) -> f32;
    fn from_f32(v: f32) -> Self;
}

impl ParamValue for f32 {
    const TOGGLE: bool = false;
    fn to_f32(self) -> f32 {
        self
    }
    fn from_f32(v: f32) -> Self {
        v
    }
}

impl ParamValue for bool {
    const TOGGLE: bool = true;
    fn to_f32(self) -> f32 {
        if self { 1.0 } else { 0.0 }
    }
    fn from_f32(v: f32) -> Self {
        v >= 0.5
    }
}

/// Declares a tunable struct. Each field is `name: type = default, [min, max]` with doc
/// comments carried over to debug tooling.
#[macro_export]
macro_rules! params {
    (
        $(#[$meta:meta])*
        pub struct $name:ident in $group:literal {
            $( $(#[doc = $doc:literal])* $field:ident : $ty:ty = $default:expr, [$min:expr, $max:expr] ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            $( $(#[doc = $doc])* pub $field: $ty, )*
        }

        impl Default for $name {
            fn default() -> Self {
                Self { $( $field: $default, )* }
            }
        }

        impl $crate::params::Params for $name {
            const GROUP: &'static str = $group;

            fn defs() -> &'static [$crate::params::ParamDef] {
                &[ $( $crate::params::ParamDef {
                    name: stringify!($field),
                    min: $min as f32,
                    max: $max as f32,
                    toggle: <$ty as $crate::params::ParamValue>::TOGGLE,
                    doc: concat!($($doc),*),
                }, )* ]
            }

            fn get(&self, name: &str) -> Option<f32> {
                match name {
                    $( stringify!($field) => Some($crate::params::ParamValue::to_f32(self.$field)), )*
                    _ => None,
                }
            }

            fn set(&mut self, name: &str, value: f32) -> bool {
                match name {
                    $( stringify!($field) => {
                        self.$field = <$ty as $crate::params::ParamValue>::from_f32(value);
                        true
                    } )*
                    _ => false,
                }
            }
        }
    };
}
