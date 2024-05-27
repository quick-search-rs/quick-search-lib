use std::collections::BTreeMap;

use abi_stable::{
    std_types::{RHashMap, ROption, RString, RVec, Tuple2},
    StableAbi,
};
use serde::{Deserialize, Serialize, Serializer};

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[sabi(impl_InterfaceType(Clone, Debug, Send, Sync, PartialEq, Eq))]
pub struct Config {
    #[serde(serialize_with = "ordered_map")]
    entries: RHashMap<RString, EntryType>,
}

fn ordered_map<S, K: Ord + Serialize, V: Serialize>(value: &RHashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().map(|Tuple2(k, v)| (k, v)).collect();
    ordered.serialize(serializer)
}

impl Config {
    pub fn new() -> Self {
        Self { entries: RHashMap::new() }
    }
    pub fn get_or_default(&self, key: &str, defaults: &Config) -> Option<EntryType> {
        self.entries.get(key).cloned().or_else(|| defaults.entries.get(key).cloned())
    }
    pub fn get(&self, key: &str) -> Option<&EntryType> {
        self.entries.get(key)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut EntryType> {
        self.entries.get_mut(key)
    }
    pub fn insert(&mut self, key: RString, value: EntryType) {
        self.entries.insert(key, value);
    }
    pub fn remove(&mut self, key: &RString) {
        self.entries.remove(key);
    }
    pub fn empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = (&RString, &EntryType)> {
        self.entries.iter().map(|Tuple2(key, value)| (key, value))
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&RString, &mut EntryType)> {
        self.entries.iter_mut().map(|Tuple2(key, value)| (key, value))
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[sabi(impl_InterfaceType(Clone, Debug, Send, Sync, PartialEq, Eq))]
pub enum EntryType {
    String {
        value: RString,
    },
    Bool {
        value: bool,
    },
    Int {
        value: i64,
        #[serde(default)]
        min: ROption<i64>,
        #[serde(default)]
        max: ROption<i64>,
    },
    Float {
        value: f64,
        #[serde(default)]
        min: ROption<f64>,
        #[serde(default)]
        max: ROption<f64>,
    },
    Enum {
        value: u8,
        #[serde(default)]
        options: RVec<EnumEntry>,
    },
    None,
}

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[sabi(impl_InterfaceType(Clone, Debug, Send, Sync, PartialEq, Eq))]
pub struct EnumEntry {
    pub value: u8,
    pub name: RString,
}

impl<T> From<(T, u8)> for EnumEntry
where
    T: Into<RString>,
{
    fn from((name, value): (T, u8)) -> Self {
        Self { value, name: name.into() }
    }
}

impl EntryType {
    pub fn as_string(&self) -> Option<&str> {
        match self {
            EntryType::String { value } => Some(value),
            _ => None,
        }
    }
    pub fn as_string_mut(&mut self) -> Option<&mut RString> {
        match self {
            EntryType::String { value } => Some(value),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            EntryType::Bool { value } => Some(*value),
            _ => None,
        }
    }
    pub fn as_bool_mut(&mut self) -> Option<&mut bool> {
        match self {
            EntryType::Bool { value } => Some(value),
            _ => None,
        }
    }
    pub fn as_int(&self) -> Option<i64> {
        match self {
            EntryType::Int { value, .. } => Some(*value),
            _ => None,
        }
    }
    pub fn as_int_mut(&mut self) -> Option<&mut i64> {
        match self {
            EntryType::Int { value, .. } => Some(value),
            _ => None,
        }
    }
    pub fn as_float(&self) -> Option<f64> {
        match self {
            EntryType::Float { value, .. } => Some(*value),
            _ => None,
        }
    }
    pub fn as_float_mut(&mut self) -> Option<&mut f64> {
        match self {
            EntryType::Float { value, .. } => Some(value),
            _ => None,
        }
    }
    pub fn as_enum(&self) -> Option<u8> {
        match self {
            EntryType::Enum { value, .. } => Some(*value),
            _ => None,
        }
    }
    pub fn as_enum_mut(&mut self) -> Option<&mut u8> {
        match self {
            EntryType::Enum { value, .. } => Some(value),
            _ => None,
        }
    }
    pub fn variant(&self) -> u32 {
        match self {
            EntryType::String { .. } => 0,
            EntryType::Bool { .. } => 1,
            EntryType::Int { .. } => 2,
            EntryType::Float { .. } => 3,
            EntryType::Enum { .. } => 4,
            EntryType::None => 5,
        }
    }
}
