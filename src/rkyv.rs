use std::{collections::BTreeMap, convert::TryFrom, iter::FromIterator};

use ordered_float::OrderedFloat;
use rkyv::{
    ser::{ScratchSpace, Serializer},
    Serialize,
};
use rkyv::{Archive, Archived, Deserialize, Fallible};

use crate::{IArray, INumber, IObject, IString};

use super::value::IValue;

#[derive(
    Debug, rkyv::Serialize, rkyv::Deserialize, rkyv::Archive, PartialOrd, PartialEq, Eq, Ord,
)]
#[archive(bound(serialize = "__S: rkyv::ser::ScratchSpace + rkyv::ser::Serializer"))]
#[archive(check_bytes)]
#[archive_attr(check_bytes(
    bound = "__C: rkyv::validation::ArchiveContext, <__C as rkyv::Fallible>::Error: rkyv::bytecheck::Error"
))]
#[archive_attr(derive(PartialEq, Eq, PartialOrd, Ord))]
pub enum ArchivableJson {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        Vec<ArchivableJson>,
    ),
    Object(
        #[omit_bounds]
        #[archive_attr(omit_bounds)]
        BTreeMap<String, ArchivableJson>,
    ),
}

#[derive(Archive, Debug, Deserialize, Serialize, PartialEq, PartialOrd, Eq, Ord)]
#[archive(check_bytes)]
#[archive_attr(derive(PartialEq, Eq, PartialOrd, Ord))]
pub enum JsonNumber {
    PosInt(u64),
    NegInt(i64),
    Float(OrderedFloat<f64>),
}

impl From<&IValue> for ArchivableJson {
    fn from(value: &IValue) -> Self {
        match value.destructure_ref() {
            crate::DestructuredRef::Null => ArchivableJson::Null,
            crate::DestructuredRef::Bool(b) => ArchivableJson::Bool(b),
            crate::DestructuredRef::Number(n) => ArchivableJson::Number({
                if n.has_decimal_point() {
                    JsonNumber::Float(n.to_f64().unwrap().into())
                } else if let Some(v) = n.to_i64() {
                    JsonNumber::NegInt(v)
                } else {
                    JsonNumber::PosInt(n.to_u64().unwrap())
                }
            }),
            crate::DestructuredRef::String(s) => ArchivableJson::String(s.to_string()),
            crate::DestructuredRef::Array(a) => {
                ArchivableJson::Array(a.into_iter().map(ArchivableJson::from).collect())
            }
            crate::DestructuredRef::Object(obj) => ArchivableJson::Object(
                obj.into_iter()
                    .map(|(k, v)| (k.to_string(), ArchivableJson::from(v)))
                    .collect(),
            ),
        }
    }
}

impl From<IValue> for ArchivableJson {
    fn from(value: IValue) -> Self {
        ArchivableJson::from(&value)
    }
}

impl From<ArchivableJson> for IValue {
    fn from(value: ArchivableJson) -> Self {
        match value {
            ArchivableJson::Null => IValue::NULL,
            ArchivableJson::Bool(b) => {
                if b {
                    IValue::TRUE
                } else {
                    IValue::FALSE
                }
            }
            ArchivableJson::Number(n) => match n {
                JsonNumber::PosInt(u) => INumber::from(u).into(),
                JsonNumber::NegInt(neg) => INumber::from(neg).into(),
                JsonNumber::Float(f) => INumber::try_from(f.into_inner())
                    .expect("unexpected float")
                    .into(),
            },
            ArchivableJson::String(s) => IString::from(s).into(),
            ArchivableJson::Array(arr) => {
                let new: Vec<IValue> = arr.into_iter().map(IValue::from).collect();
                IArray::from(new).into()
            }
            ArchivableJson::Object(obj) => IObject::from_iter(obj).into(),
        }
    }
}

impl<S: Serializer + ScratchSpace> Serialize<S> for IValue {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivableJson::from(self).serialize(serializer)
    }
}

impl Archive for IValue {
    type Archived = <ArchivableJson as Archive>::Archived;
    type Resolver = <ArchivableJson as Archive>::Resolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        ArchivableJson::from(self).resolve(pos, resolver, out)
    }
}

impl<D: Fallible + ?Sized> Deserialize<IValue, D> for Archived<IValue> {
    fn deserialize(&self, deserializer: &mut D) -> Result<IValue, D::Error> {
        let r: ArchivableJson = rkyv::Deserialize::deserialize(self, deserializer)?;

        Ok(IValue::from(r))
    }
}

impl<S: Serializer + ScratchSpace> Serialize<S> for IString {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        self.to_string().serialize(serializer)
    }
}

impl Archive for IString {
    type Archived = <String as Archive>::Archived;
    type Resolver = <String as Archive>::Resolver;

    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        self.to_string().resolve(pos, resolver, out)
    }
}

impl<D: Fallible + ?Sized> Deserialize<IString, D> for Archived<IString> {
    fn deserialize(&self, deserializer: &mut D) -> Result<IString, D::Error> {
        let r: String = rkyv::Deserialize::deserialize(self, deserializer)?;

        Ok(IString::from(r))
    }
}

#[cfg(test)]
mod tests {
    use rkyv::Deserialize;

    use crate::{IString, IValue};

    use super::ArchivableJson;

    #[test]
    fn test_serialization() {
        let x: IValue = serde_json::from_str(
            r#"
        {
            "songs": [
                {
                  "title": "Fairies Wear Boots",
                  "artist": "Black Sabbath",
                  "album": "Paranoid",
                  "release_year": 1970,
                  "genre": ["Heavy Metal", "Hard Rock"]
                },
                {
                  "title": "Whole Lotta Love",
                  "artist": "Led Zeppelin",
                  "album": "Led Zeppelin II",
                  "release_year": 1969,
                  "genre": ["Hard Rock", "Blues Rock"]
                },
                {
                  "title": "Hysteria",
                  "artist": "Muse",
                  "album": "Absolution",
                  "release_year": 2003,
                  "genre": ["Alternative Rock", "Art Rock"]
                },
                {
                  "title": "Bohemian Rhapsody",
                  "artist": "Queen",
                  "album": "A Night at the Opera",
                  "release_year": 1975,
                  "genre": ["Progressive Rock", "Symphonic Rock"]
                },
                {
                  "title": "Hotel California",
                  "artist": "Eagles",
                  "album": "Hotel California",
                  "release_year": 1976,
                  "genre": ["Rock", "Soft Rock"]
                },
                {
                  "title": "Smells Like Teen Spirit",
                  "artist": "Nirvana",
                  "album": "Nevermind",
                  "release_year": 1991,
                  "genre": ["Grunge", "Alternative Rock"]
                },
                {
                  "title": "Stairway to Heaven",
                  "artist": "Led Zeppelin",
                  "album": "Led Zeppelin IV",
                  "release_year": 1971,
                  "genre": ["Hard Rock", "Folk Rock"]
                },
                {
                  "title": "Imagine",
                  "artist": "John Lennon",
                  "album": "Imagine",
                  "release_year": 1971,
                  "genre": ["Soft Rock", "Pop"]
                },
                {
                  "title": "Yesterday",
                  "artist": "The Beatles",
                  "album": "Help!",
                  "release_year": 1965,
                  "genre": ["Folk Rock", "Baroque Pop"]
                }
            ]
        }
        "#,
        )
        .unwrap();

        let encoded = rkyv::to_bytes::<_, 256>(&x).unwrap();
        let archived = unsafe { rkyv::archived_root::<ArchivableJson>(&encoded[..]) };
        let decoded: IValue = archived.deserialize(&mut rkyv::Infallible).unwrap();

        assert_eq!(x, decoded);
    }

    #[test]
    fn test_string_serialization() {
        let s = "hello".to_string();

        let encoded = rkyv::to_bytes::<_, 256>(&s).unwrap();
        let archived = unsafe { rkyv::archived_root::<String>(&encoded[..]) };
        let decoded: IString = archived.deserialize(&mut rkyv::Infallible).unwrap();

        assert_eq!(s, decoded);
    }
}
