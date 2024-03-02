// shared logging module with abi stable types to be used within the plugin system

use abi_stable::{
    external_types::{
        crossbeam_channel::{self, RReceiver, RSender},
        RMutex,
    },
    std_types::{RArc, RString},
    StableAbi,
};
use serde::{Deserialize, Serialize};

pub trait Log {
    fn log(&self, message: &str, level: LogLevel) {
        if self.log_level().is_enabled(level) {
            let message = LogMessage {
                message: RArc::new(message.into()),
                level,
                source: self.source(),
                time: U128Wrapper::new(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()),
            };
            if self.send(message).is_err() {
                eprintln!("Error sending log message") // kinda meta having a log message about a log message failing lol but i dont want to do anything else here
            };
        }
    }
    fn debug(&self, message: &str) {
        self.log(message, LogLevel::Debug);
    }
    fn info(&self, message: &str) {
        self.log(message, LogLevel::Info);
    }
    fn warn(&self, message: &str) {
        self.log(message, LogLevel::Warn);
    }
    fn error(&self, message: &str) {
        self.log(message, LogLevel::Error);
    }
    fn trace(&self, message: &str) {
        self.log(message, LogLevel::Trace);
    }
    fn log_level(&self) -> LogLevelOrCustom;
    fn source(&self) -> RArc<RString>;
    fn send(&self, message: LogMessage) -> Result<(), LogMessage>;
}

// main struct for logging, keeps a list of all pending log messages and handles receiving new log messages
#[repr(C)]
#[derive(StableAbi)]
pub struct Logger {
    messages: RReceiver<LogMessage>,
    _log_level: RArc<RMutex<LogLevelOrCustom>>,
    _sender: RSender<LogMessage>,
    _source: RArc<RString>,
}

impl Log for Logger {
    fn log_level(&self) -> LogLevelOrCustom {
        *self._log_level.lock()
    }
    fn source(&self) -> RArc<RString> {
        RArc::clone(&self._source)
    }
    fn send(&self, message: LogMessage) -> Result<(), LogMessage> {
        self._sender.send(message).map_err(|e| e.0)
    }
}

impl Logger {
    pub fn new(log_level: LogLevelOrCustom) -> Self {
        let (_sender, messages) = crossbeam_channel::unbounded();
        Self {
            messages,
            _log_level: RArc::new(RMutex::new(log_level)),
            _sender,
            _source: RArc::new("raw".into()),
        }
    }
    pub fn new_scoped(&self, source: &str) -> ScopedLogger {
        ScopedLogger::new(RArc::clone(&self._log_level), source, RSender::clone(&self._sender))
    }
    pub fn set_log_level(&self, log_level: LogLevelOrCustom) {
        *self._log_level.lock() = log_level;
    }
    pub fn get(&self) -> Vec<LogMessage> {
        let mut messages = Vec::new();
        while let Ok(message) = self.messages.try_recv() {
            messages.push(message);
        }
        messages
    }
}

#[repr(C)]
#[derive(StableAbi)]
pub struct ScopedLogger {
    _log_level: RArc<RMutex<LogLevelOrCustom>>,
    _source: RArc<RString>,
    _sender: RSender<LogMessage>,
}

impl Log for ScopedLogger {
    fn log_level(&self) -> LogLevelOrCustom {
        *self._log_level.lock()
    }
    fn source(&self) -> RArc<RString> {
        RArc::clone(&self._source)
    }
    fn send(&self, message: LogMessage) -> Result<(), LogMessage> {
        self._sender.send(message).map_err(|e| e.0)
    }
}

impl ScopedLogger {
    pub fn new(_log_level: RArc<RMutex<LogLevelOrCustom>>, source: &str, sender: RSender<LogMessage>) -> Self {
        Self {
            _log_level,
            _source: RArc::new(source.into()),
            _sender: sender,
        }
    }
}

#[repr(C)]
#[derive(StableAbi, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum LogLevelOrCustom {
    LogLevel(LogLevel),
    Custom(LogLevelBitmask),
}

impl LogLevelOrCustom {
    // check if a log at the given level should actually be stored
    fn is_enabled(&self, level: LogLevel) -> bool {
        match self {
            LogLevelOrCustom::LogLevel(l) => *l as u8 >= level as u8,
            LogLevelOrCustom::Custom(mask) => mask.0 & level as u8 != 0,
        }
    }
    // create a new LogLevelOrCustom from a LogLevel as the minimum level
    pub fn from_min_level(level: LogLevel) -> Self {
        LogLevelOrCustom::LogLevel(level)
    }
    // create a new LogLevelOrCustom from a list of LogLevel as the selected levels
    pub fn from_levels(levels: &[LogLevel]) -> Self {
        let mut mask = 0;
        for level in levels {
            mask |= *level as u8;
        }
        LogLevelOrCustom::Custom(LogLevelBitmask(mask))
    }
}

#[repr(u8)]
#[derive(StableAbi, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 16,
    Info = 8,
    Warn = 4,
    Error = 2,
    Trace = 1,
}

#[repr(transparent)]
#[derive(StableAbi, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct LogLevelBitmask(u8);

impl LogLevelBitmask {
    pub fn mask(&self) -> u8 {
        self.0
    }
    pub fn from_mask(mask: u8) -> Self {
        Self(mask)
    }
}

// main struct for log messages, keeps the message, the level, the source, and the time it was received
#[repr(C)]
#[derive(StableAbi)]
pub struct LogMessage {
    pub message: RArc<RString>,
    pub level: LogLevel,
    pub source: RArc<RString>,
    pub time: U128Wrapper,
}

#[repr(C)]
#[derive(StableAbi, Clone, Copy, PartialEq, Eq, Debug)]
pub struct U128Wrapper {
    first: u64,
    second: u64,
}

impl U128Wrapper {
    pub fn new(value: u128) -> Self {
        Self {
            first: (value >> 64) as u64,
            second: value as u64,
        }
    }
    pub fn get(&self) -> u128 {
        (self.first as u128) << 64 | self.second as u128
    }
}

impl std::cmp::PartialOrd for U128Wrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for U128Wrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.get().cmp(&other.get())
    }
}
