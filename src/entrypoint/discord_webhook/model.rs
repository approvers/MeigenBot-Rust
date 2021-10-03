use std::fmt::{Formatter, Result as FmtResult};

use serde::{
    de::{Deserialize, Deserializer, Error, MapAccess, Visitor},
    Deserialize as DeserializeMacro,
};

#[derive(DeserializeMacro)]
pub(super) struct Request {
    pub(super) data: RequestData,
    pub(super) member: RequestMember,
}

#[derive(DeserializeMacro)]
pub(super) struct RequestMember {
    pub(super) user: RequestUser,
}

#[derive(DeserializeMacro)]
pub(super) struct RequestUser {
    pub(super) id: String,
}

#[derive(DeserializeMacro)]
pub(super) struct RequestData {
    pub(super) options: Vec<RequestOption>,
}

pub(super) struct RequestOption {
    #[allow(unused)]
    pub(super) ty: Option<i32>,
    pub(super) name: String,
    pub(super) value: Option<String>,
    pub(super) options: Option<Vec<RequestOption>>,
}

const REQUEST_OPTION_FIELDS: &[&str] = &["name", "value", "options"];

impl<'d> serde::de::Deserialize<'d> for RequestOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        deserializer.deserialize_struct(
            "RequestOption",
            REQUEST_OPTION_FIELDS,
            RequestOptionVisitor,
        )
    }
}

enum RequestOptionField {
    Name,
    Value,
    Options,
    Type,
}

impl<'d> serde::de::Deserialize<'d> for RequestOptionField {
    fn deserialize<D>(deserializer: D) -> Result<RequestOptionField, D::Error>
    where
        D: Deserializer<'d>,
    {
        deserializer.deserialize_identifier(RequestOptionFieldVisitor)
    }
}

struct RequestOptionFieldVisitor;

impl<'d> Visitor<'d> for RequestOptionFieldVisitor {
    type Value = RequestOptionField;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "type, name, value or options")
    }

    fn visit_str<E>(self, value: &str) -> Result<RequestOptionField, E>
    where
        E: Error,
    {
        match value {
            "name" => Ok(RequestOptionField::Name),
            "value" => Ok(RequestOptionField::Value),
            "options" => Ok(RequestOptionField::Options),
            "type" => Ok(RequestOptionField::Type),
            _ => Err(Error::unknown_field(value, REQUEST_OPTION_FIELDS)),
        }
    }
}

struct RequestOptionVisitor;

impl<'d> Visitor<'d> for RequestOptionVisitor {
    type Value = RequestOption;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "struct RequestOption")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'d>,
    {
        let mut name: Option<String> = None;
        let mut value: Option<RequestOptionValue> = None;
        let mut options: Option<Vec<RequestOption>> = None;
        let mut ty: Option<i32> = None;

        while let Some(key) = map.next_key::<RequestOptionField>()? {
            match key {
                RequestOptionField::Name => {
                    if name.is_some() {
                        return Err(Error::duplicate_field("name"));
                    }

                    name = Some(map.next_value()?)
                }

                RequestOptionField::Value => {
                    if value.is_some() {
                        return Err(Error::duplicate_field("value"));
                    }

                    value = Some(map.next_value()?);
                }

                RequestOptionField::Options => {
                    if options.is_some() {
                        return Err(Error::duplicate_field("options"));
                    }

                    options = Some(map.next_value()?);
                }

                RequestOptionField::Type => {
                    if ty.is_some() {
                        return Err(Error::duplicate_field("type"));
                    }

                    ty = Some(map.next_value()?);
                }
            }
        }

        let name = name.ok_or_else(|| Error::missing_field("name"))?;

        Ok(RequestOption {
            ty,
            name,
            value: value.map(|x| x.0),
            options,
        })
    }
}

// this is special deserializable struct.
// this deserializes string, but also parses number as string.
struct RequestOptionValue(String);
impl<'d> Deserialize<'d> for RequestOptionValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        deserializer.deserialize_any(RequestOptionValueVisitor)
    }
}

struct RequestOptionValueVisitor;
impl<'d> Visitor<'d> for RequestOptionValueVisitor {
    type Value = RequestOptionValue;

    fn expecting(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "number or string")
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RequestOptionValue(v.to_string()))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RequestOptionValue(v.to_string()))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RequestOptionValue(v.to_string()))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RequestOptionValue(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(RequestOptionValue(v))
    }
}
