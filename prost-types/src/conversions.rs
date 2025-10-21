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

impl<'arena> prost::ArenaFrom<'arena, String> for Value<'arena> {
    fn arena_from(value: String, arena: &'arena prost::Arena) -> Self {
        let s = arena.alloc_str(&value);
        value::Kind::StringValue(s).into()
    }
}

impl<'arena> prost::ArenaFrom<'arena, &str> for Value<'arena> {
    fn arena_from(value: &str, arena: &'arena prost::Arena) -> Self {
        let s = arena.alloc_str(value);
        value::Kind::StringValue(s).into()
    }
}

impl<'arena> prost::ArenaFrom<'arena, Vec<Value<'arena>>> for Value<'arena> {
    fn arena_from(values: Vec<Value<'arena>>, arena: &'arena prost::Arena) -> Self {
        let values_slice = arena.alloc_slice_fill_iter(values.into_iter());
        let list_value = arena.alloc(crate::protobuf::ListValue { values: values_slice });
        value::Kind::ListValue(list_value).into()
    }
}

impl<'arena> prost::ArenaFrom<'arena, BTreeMap<String, Value<'arena>>> for Value<'arena> {
    fn arena_from(map: BTreeMap<String, Value<'arena>>, arena: &'arena prost::Arena) -> Self {
        // Convert BTreeMap to iterator of tuples, then allocate as slice
        let entries = map.into_iter();
        let fields_slice = arena.alloc_slice_fill_iter(entries);
        let struct_value = arena.alloc(crate::protobuf::Struct {
            fields: prost::ArenaMap::new(fields_slice)
        });
        value::Kind::StructValue(struct_value).into()
    }
}
