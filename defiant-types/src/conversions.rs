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
        let mut vec = arena.new_vec();
        vec.extend(values);
        let values_slice = vec.freeze();
        let list_value = arena.alloc(crate::protobuf::ListValue { values: values_slice });
        value::Kind::ListValue(list_value).into()
    }
}

impl<'arena> prost::ArenaFrom<'arena, BTreeMap<String, Value<'arena>>> for Value<'arena> {
    fn arena_from(map: BTreeMap<String, Value<'arena>>, arena: &'arena prost::Arena) -> Self {
        // Convert BTreeMap to arena map, allocating String keys into arena as &str
        let mut vec: prost::ArenaVec<'arena, (&'arena str, Value<'arena>)> = arena.new_vec();
        for (k, v) in map.into_iter() {
            let key_ref = arena.alloc_str(&k);
            vec.push((key_ref, v));
        }
        let fields_slice = vec.freeze();
        let struct_value = arena.alloc(crate::protobuf::Struct {
            fields: prost::ArenaMap::new(fields_slice)
        });
        value::Kind::StructValue(struct_value).into()
    }
}
