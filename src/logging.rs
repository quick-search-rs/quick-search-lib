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
    fn import_deserialize(&self, message: &str);
}

// main struct for logging, keeps a list of all pending log messages and handles receiving new log messages
#[repr(C)]
#[derive(StableAbi)]
pub struct Logger {
    messages: RReceiver<LogMessage>,
    log_level: RArc<RMutex<LogLevelOrCustom>>,
    sender: RSender<LogMessage>,
    source: RArc<RString>,
    stdout: bool,
}

impl Log for Logger {
    fn log_level(&self) -> LogLevelOrCustom {
        *self.log_level.lock()
    }
    fn source(&self) -> RArc<RString> {
        RArc::clone(&self.source)
    }
    fn send(&self, message: LogMessage) -> Result<(), LogMessage> {
        if self.stdout {
            println!("{}", serde_json::to_string(&message).unwrap_or_default());
        }
        self.sender.send(message).map_err(|e| e.0)
    }
    fn import_deserialize(&self, message: &str) {
        if message.is_empty() {
            return;
        }
        // attempt to deserialize the message
        if let Ok(message) = serde_json::from_str::<LogMessage>(message) {
            // if the message is deserialized successfully, send it
            if self.send(message).is_err() {
                eprintln!("Error sending log message") // kinda meta having a log message about a log message failing lol but i dont want to do anything else here
            };
        } else {
            // if the message fails to deserialize, log an error
            self.error(&format!("Failed to deserialize log message: {}", message));
        }
    }
}

impl Logger {
    pub fn new(log_level: LogLevelOrCustom, stdout: bool) -> Self {
        let (sender, messages) = crossbeam_channel::unbounded();
        Self {
            messages,
            log_level: RArc::new(RMutex::new(log_level)),
            sender,
            source: RArc::new("raw".into()),
            stdout,
        }
    }
    pub fn new_scoped(&self, source: &str) -> ScopedLogger {
        ScopedLogger::new(RArc::clone(&self.log_level), source, RSender::clone(&self.sender), self.stdout)
    }
    pub fn set_log_level(&self, log_level: LogLevelOrCustom) {
        *self.log_level.lock() = log_level;
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
    log_level: RArc<RMutex<LogLevelOrCustom>>,
    source: RArc<RString>,
    sender: RSender<LogMessage>,
    stdout: bool,
}

impl Log for ScopedLogger {
    fn log_level(&self) -> LogLevelOrCustom {
        *self.log_level.lock()
    }
    fn source(&self) -> RArc<RString> {
        RArc::clone(&self.source)
    }
    fn send(&self, message: LogMessage) -> Result<(), LogMessage> {
        if self.stdout {
            println!("{}", serde_json::to_string(&message).unwrap_or_default());
        }
        self.sender.send(message).map_err(|e| e.0)
    }
    fn import_deserialize(&self, message: &str) {
        if message.is_empty() {
            return;
        }
        // attempt to deserialize the message
        if let Ok(message) = serde_json::from_str::<LogMessage>(message) {
            // if the message is deserialized successfully, send it
            if self.send(message).is_err() {
                eprintln!("Error sending log message") // kinda meta having a log message about a log message failing lol but i dont want to do anything else here
            };
        } else {
            // if the message fails to deserialize, log an error
            self.error(&format!("Failed to deserialize log message: {}", message));
        }
    }
}

impl ScopedLogger {
    pub fn new(log_level: RArc<RMutex<LogLevelOrCustom>>, source: &str, sender: RSender<LogMessage>, stdout: bool) -> Self {
        Self {
            log_level,
            source: RArc::new(source.into()),
            sender,
            stdout,
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
    Trace = 1,
    Debug = 2,
    Info = 4,
    Warn = 8,
    Error = 16,
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
#[derive(StableAbi, Serialize, Deserialize)]
pub struct LogMessage {
    pub message: RArc<RString>,
    pub level: LogLevel,
    pub source: RArc<RString>,
    #[serde(with = "u128_wrapper")]
    pub time: U128Wrapper,
}

mod u128_wrapper {
    use super::U128Wrapper;
    use serde::{de::Visitor, Deserializer, Serializer};

    pub fn serialize<S>(value: &U128Wrapper, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u128(value.get())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<U128Wrapper, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = U128WrapperVisitor;
        Ok(U128Wrapper::new(deserializer.deserialize_u128(visitor)?))
    }

    struct U128WrapperVisitor;

    impl<'de> Visitor<'de> for U128WrapperVisitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a u128")
        }

        fn visit_u128<E>(self, value: u128) -> Result<u128, E> {
            Ok(value)
        }
    }
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
