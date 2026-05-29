use serde::{Deserializer, Serializer};
use std::str::FromStr;

pub fn serialize_vector_as_string<T, S>(vec: &[T], s: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    let mut string = String::new();

    for item in vec {
        string.push_str(&item.to_string());
        string.push(',');
    }
    string.pop();

    s.serialize_str(&string)
}

pub fn serialize_vector_as_string_opt<'a, I, T, S>(
    vec: &'a Option<I>,
    s: S,
) -> Result<S::Ok, S::Error>
where
    &'a I: IntoIterator<Item = T> + 'a,
    T: ToString + 'a,
    S: Serializer,
{
    match vec {
        Some(vec) => {
            let mut string = String::new();

            for item in vec {
                string.push_str(&item.to_string());
                string.push(',');
            }
            string.pop();

            s.serialize_some(&string)
        }
        None => s.serialize_none(),
    }
}

pub fn serialize_as_string_opt<T, S>(t: &Option<T>, s: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    match *t {
        Some(ref t) => s.serialize_some(&t.to_string()),
        None => s.serialize_none(),
    }
}

pub fn serialize_bool_as_string<S>(boolean: &bool, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match boolean {
        true => s.serialize_str("1"),
        false => s.serialize_str("0"),
    }
}

pub fn serialize_as_string<T, S>(t: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: ToString,
    S: Serializer,
{
    s.serialize_str(&t.to_string())
}

pub fn deserialize_from_string<'de, T, D>(d: D) -> Result<T, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
{
    d.deserialize_string(string_visitor::FromStrVisitor::<T>::default())
}

pub fn deserialize_from_string_opt<'de, T, D>(d: D) -> Result<Option<T>, D::Error>
where
    T: FromStr,
    D: Deserializer<'de>,
{
    d.deserialize_option(opt_string_visitor::OptionFromStrVisitor::<T>::default())
}

mod string_visitor {
    use serde::de::{Error, Unexpected, Visitor};
    use std::fmt;
    use std::marker::PhantomData;
    use std::str;
    use std::str::FromStr;

    pub struct FromStrVisitor<T: FromStr> {
        _marker: PhantomData<T>,
    }

    impl<T: FromStr> Default for FromStrVisitor<T> {
        fn default() -> Self {
            Self {
                _marker: PhantomData::default(),
            }
        }
    }

    impl<'de, T: FromStr> Visitor<'de> for FromStrVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a type that parses from a string")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            v.parse()
                .map_err(|_| E::custom(format!("cannot be parsed: {}", v)))
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: Error,
        {
            v.parse()
                .map_err(|_| E::custom(format!("cannot be parsed: {}", v)))
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
        where
            E: Error,
        {
            match str::from_utf8(v) {
                Ok(s) => s
                    .parse()
                    .map_err(|_| E::custom(format!("cannot be parsed: {}", s))),
                Err(_) => Err(Error::invalid_value(Unexpected::Bytes(v), &self)),
            }
        }
    }
}

mod opt_string_visitor {
    use serde::de::{Deserializer, Error, Visitor};
    use std::fmt;
    use std::marker::PhantomData;
    use std::str::FromStr;

    pub struct OptionFromStrVisitor<T: FromStr> {
        _marker: PhantomData<T>,
    }

    impl<T: FromStr> Default for OptionFromStrVisitor<T> {
        fn default() -> Self {
            Self {
                _marker: PhantomData::default(),
            }
        }
    }

    impl<'de, T: FromStr> Visitor<'de> for OptionFromStrVisitor<T> {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an optional type that parses from a string")
        }

        #[inline]
        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        #[inline]
        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(None)
        }

        #[inline]
        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            super::deserialize_from_string(deserializer).map(Some)
        }
    }
}
