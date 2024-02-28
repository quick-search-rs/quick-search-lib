#![allow(dead_code)]
#![allow(non_camel_case_types)]

mod chars;

use std::{collections::BTreeMap, path::Path};

pub use chars::ColoredChar;

use abi_stable::{
    library::{LibraryError, RootModule},
    package_version_strings, sabi_trait,
    std_types::{RBox, RCowStr, RHashMap, ROption, RStr, RString, RVec, Tuple2},
    StableAbi,
};

pub use abi_stable;
use serde::{Deserialize, Serialize, Serializer};

#[sabi_trait]
pub trait Searchable: Send + Sync + Clone {
    fn search(&self, query: RString) -> RVec<SearchResult>;
    fn name(&self) -> RStr<'static>;
    fn colored_name(&self) -> RVec<ColoredChar>;
    fn execute(&self, selected_result: &SearchResult);
    fn plugin_id(&self) -> PluginId;

    // config related
    // will be called with EntryType containing the user configured values
    fn lazy_load_config(&mut self, config: Config) {
        let _ = config;
    }
    // when called, the contained values should be the defaults the plugin wants, will be called every time to ensure the config is valid and retrieve default values if it is malformed
    fn get_config_entries(&self) -> Config {
        Config { entries: RHashMap::new() }
    }
}

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
        }
    }
}

#[repr(C)]
#[derive(StableAbi, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[sabi(impl_InterfaceType(Clone, Debug, Send, Sync, PartialEq, Eq))]
pub struct SearchResult {
    title: RString,
    context: RString,
    extra_info: RString,
}

pub type SearchableBox = Searchable_TO<'static, RBox<()>>;

impl SearchResult {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.into(),
            context: "".into(),
            extra_info: "".into(),
        }
    }
    pub fn set_title(mut self, title: &str) -> Self {
        self.title = title.into();
        self
    }
    pub fn set_context(mut self, context: &str) -> Self {
        self.context = context.into();
        self
    }
    pub fn set_extra_info(mut self, extra_info: &str) -> Self {
        self.extra_info = extra_info.into();
        self
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn context(&self) -> &str {
        &self.context
    }
    pub fn extra_info(&self) -> &str {
        &self.extra_info
    }
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix(prefix_ref = SearchLib_Ref)))]
#[sabi(missing_field(panic))]
pub struct SearchLib {
    #[sabi(last_prefix_field)]
    pub get_searchable: extern "C" fn(PluginId) -> SearchableBox,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq, StableAbi)]
pub struct PluginId {
    pub filename: RCowStr<'static>,
}

impl RootModule for SearchLib_Ref {
    abi_stable::declare_root_module_statics! {SearchLib_Ref}

    const BASE_NAME: &'static str = "search_libs";
    const NAME: &'static str = "search_libs";
    const VERSION_STRINGS: abi_stable::sabi_types::VersionStrings = package_version_strings!();
}

pub fn load_library(path: &Path) -> Result<SearchLib_Ref, LibraryError> {
    abi_stable::library::lib_header_from_path(path).and_then(|x| x.init_root_module::<SearchLib_Ref>())
    // SearchLib_Ref::load_from_file(path)
}
