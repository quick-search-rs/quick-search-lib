#![allow(dead_code, non_camel_case_types, clippy::empty_docs)]

mod chars;
mod config;
mod logging;

use std::path::{Path, PathBuf};

pub use chars::*;
pub use config::*;
pub use logging::*;

use abi_stable::{
    library::{LibraryError, RootModule},
    package_version_strings, sabi_trait,
    std_types::{RBox, RCowStr, RStr, RString, RVec},
    StableAbi,
};

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

type SearchableBox = Searchable_TO<'static, RBox<()>>;

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

// ORDERING IS IMPORTANT, WE NEED THE FIELDS TO DROP IN THIS ORDER:
// 1. searchable
// 2. lib
// 3. raw_lib
pub struct SearchableLibrary {
    path: PathBuf,
    searchable: Option<SearchableBox>,
    #[cfg(not(feature = "leaky-loader"))]
    raw_lib: Option<abi_stable::library::RawLibrary>,
}

impl SearchableLibrary {
    pub fn new(path: PathBuf, logger: ScopedLogger) -> Result<Self, LibraryError> {
        #[cfg(not(feature = "leaky-loader"))]
        let raw_lib = abi_stable::library::RawLibrary::load_at(&path)?;
        #[cfg(feature = "leaky-loader")]
        {
            check_library(&path)?;
        }
        Ok(Self {
            searchable: Some({
                #[cfg(not(feature = "leaky-loader"))]
                {
                    Self::load(&raw_lib)?
                }
                #[cfg(feature = "leaky-loader")]
                {
                    load_library(&path)?
                }
            }
            .get_searchable()(
                PluginId {
                    filename: {
                        path.file_name()
                            .ok_or(LibraryError::RootModule {
                                err: abi_stable::library::RootModuleError::Unwound,
                                module_name: "SearchLib_Ref",
                                version: package_version_strings!(),
                            })?
                            .to_string_lossy()
                            .into_owned()
                            .into()
                    },
                },
                logger,
            )),
            #[cfg(not(feature = "leaky-loader"))]
            raw_lib: Some(raw_lib),
            path,
        })
    }
    #[cfg(not(feature = "leaky-loader"))]
    fn load(raw_lib: &abi_stable::library::RawLibrary) -> Result<SearchLib_Ref, LibraryError> {
        unsafe { abi_stable::library::lib_header_from_raw_library(raw_lib) }.and_then(|x| x.init_root_module::<SearchLib_Ref>())
    }
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.search(query.into()).into()
    }
    pub fn name(&self) -> &str {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.name().into()
    }
    pub fn colored_name(&self) -> Vec<ColoredChar> {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.colored_name().into()
    }
    pub fn execute(&self, selected_result: &SearchResult) {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.execute(selected_result);
    }
    pub fn plugin_id(&self) -> PluginId {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.plugin_id()
    }
    pub fn lazy_load_config(&mut self, config: Config) {
        unsafe { self.searchable.as_mut().unwrap_unchecked() }.lazy_load_config(config);
    }
    pub fn get_config_entries(&self) -> Config {
        unsafe { self.searchable.as_ref().unwrap_unchecked() }.get_config_entries()
    }
}

impl Drop for SearchableLibrary {
    fn drop(&mut self) {
        #[cfg(feature = "debug")]
        eprintln!("Dropping SearchableLibrary: {:?}", self.path);
        std::mem::drop(self.searchable.take());
        #[cfg(not(feature = "leaky-loader"))]
        std::mem::drop(self.raw_lib.take());
    }
}

fn load_library(path: &Path) -> Result<SearchLib_Ref, LibraryError> {
    abi_stable::library::lib_header_from_path(path).and_then(|x| x.init_root_module::<SearchLib_Ref>())
    // SearchLib_Ref::load_from_file(path)
}

fn check_library(path: &Path) -> Result<(), LibraryError> {
    let raw_library = abi_stable::library::RawLibrary::load_at(path)?;
    unsafe { abi_stable::library::lib_header_from_raw_library(&raw_library) }.and_then(|x| x.check_layout::<SearchLib_Ref>())?;
    Ok(())
}
