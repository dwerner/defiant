// Extension traits to provide prost-style API on defiant descriptor types

use defiant_types::field_descriptor_proto::{Label, Type};
use defiant_types::{
    DescriptorProto, EnumDescriptorProto, EnumValueDescriptorProto, EnumValueOptions,
    FieldDescriptorProto, FieldOptions, FileDescriptorProto, MethodDescriptorProto,
    OneofDescriptorProto, ServiceDescriptorProto,
};

pub trait FieldDescriptorProtoExt {
    fn name(&self) -> &str;
    fn number(&self) -> i32;
    fn label(&self) -> Label;
    fn r#type(&self) -> Type;
    fn type_name(&self) -> &str;
}

impl<'arena> FieldDescriptorProtoExt for FieldDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
    fn number(&self) -> i32 {
        self.number.unwrap_or(0)
    }
    fn label(&self) -> Label {
        self.label
            .and_then(|l| Label::try_from(l).ok())
            .unwrap_or(Label::Optional)
    }
    fn r#type(&self) -> Type {
        self.r#type
            .and_then(|t| Type::try_from(t).ok())
            .unwrap_or(Type::Double)
    }
    fn type_name(&self) -> &str {
        self.type_name.unwrap_or("")
    }
}

pub trait EnumValueDescriptorProtoExt {
    fn name(&self) -> &str;
    fn number(&self) -> i32;
}

impl<'arena> EnumValueDescriptorProtoExt for EnumValueDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
    fn number(&self) -> i32 {
        self.number.unwrap_or(0)
    }
}

pub trait DescriptorProtoExt {
    fn name(&self) -> &str;
}

impl<'arena> DescriptorProtoExt for DescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
}

pub trait OneofDescriptorProtoExt {
    fn name(&self) -> &str;
}

impl<'arena> OneofDescriptorProtoExt for OneofDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
}

pub trait EnumDescriptorProtoExt {
    fn name(&self) -> &str;
}

impl<'arena> EnumDescriptorProtoExt for EnumDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
}

pub trait FileDescriptorProtoExt {
    fn package(&self) -> &str;
}

impl<'arena> FileDescriptorProtoExt for FileDescriptorProto<'arena> {
    fn package(&self) -> &str {
        self.package.unwrap_or("")
    }
}

pub trait FieldOptionsExt {
    fn packed(&self) -> bool;
    fn deprecated(self) -> bool;
}

impl<'arena> FieldOptionsExt for &FieldOptions<'arena> {
    fn packed(&self) -> bool {
        self.packed.unwrap_or(false)
    }
    fn deprecated(self) -> bool {
        self.deprecated.unwrap_or(false)
    }
}

pub trait EnumValueOptionsExt {
    fn deprecated(self) -> bool;
}

impl<'arena> EnumValueOptionsExt for &EnumValueOptions<'arena> {
    fn deprecated(self) -> bool {
        self.deprecated.unwrap_or(false)
    }
}

pub trait ServiceDescriptorProtoExt {
    fn name(&self) -> &str;
}

impl<'arena> ServiceDescriptorProtoExt for ServiceDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
}

pub trait MethodDescriptorProtoExt {
    fn name(&self) -> &str;
    fn client_streaming(&self) -> bool;
    fn server_streaming(&self) -> bool;
}

impl<'arena> MethodDescriptorProtoExt for MethodDescriptorProto<'arena> {
    fn name(&self) -> &str {
        self.name.unwrap_or("")
    }
    fn client_streaming(&self) -> bool {
        self.client_streaming.unwrap_or(false)
    }
    fn server_streaming(&self) -> bool {
        self.server_streaming.unwrap_or(false)
    }
}
