#![allow(dead_code)]
#![allow(non_camel_case_types)]

mod chars;
mod config;
mod logging;

use std::path::Path;

pub use chars::*;
pub use config::*;
pub use logging::*;

use abi_stable::{
    library::{LibraryError, RootModule},
    package_version_strings, sabi_trait,
    std_types::{RBox, RCowStr, RStr, RString, RVec},
    StableAbi,
};

pub use abi_stable;
use serde::{Deserialize, Serialize};

#[sabi_trait]
pub trait Searchable: Send + Sync {
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
        Config::default()
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
    pub get_searchable: extern "C" fn(PluginId, ScopedLogger) -> SearchableBox,
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
