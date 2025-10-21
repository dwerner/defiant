use crate::protobuf::Value;
use crate::value;
use crate::String;
use crate::Vec;
use ::prost::alloc::collections::BTreeMap;

impl<'arena> From<value::Kind<'arena>> for Value<'arena> {
    fn from(value: value::Kind<'arena>) -> Self {
        Value { kind: Some(value) }
    }
}

macro_rules! impl_number_value {
    ($t: ty) => {
        impl<'arena> From<$t> for Value<'arena> {
            fn from(value: $t) -> Self {
                value::Kind::NumberValue(value.into()).into()
            }
        }
    };
}

impl_number_value!(u8);
impl_number_value!(u16);
impl_number_value!(u32);

impl_number_value!(i8);
impl_number_value!(i16);
impl_number_value!(i32);

impl_number_value!(f32);
impl_number_value!(f64);

impl<'arena> From<bool> for Value<'arena> {
    fn from(value: bool) -> Self {
        value::Kind::BoolValue(value).into()
    }
}

impl<'arena> From<String> for Value<'arena> {
    fn from(value: String) -> Self {
        value::Kind::StringValue(value).into()
    }
}

impl<'arena> From<&str> for Value<'arena> {
    fn from(value: &str) -> Self {
        value::Kind::StringValue(value.into()).into()
    }
}

impl<'arena> From<Vec<Value<'arena>>> for Value<'arena> {
    fn from(value: Vec<Value<'arena>>) -> Self {
        value::Kind::ListValue(crate::protobuf::ListValue { values: value }).into()
    }
}

impl<'arena> From<BTreeMap<String, Value<'arena>>> for Value<'arena> {
    fn from(value: BTreeMap<String, Value<'arena>>) -> Self {
        value::Kind::StructValue(crate::protobuf::Struct { fields: value }).into()
    }
}
