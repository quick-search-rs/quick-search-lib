use abi_stable::{std_types::RVec, StableAbi};
use std::fmt::{Debug, Display, Formatter, Result};

#[repr(C)]
#[derive(StableAbi)]
pub struct ColoredChar {
    char: u32,
    color: u32,
}

impl Debug for ColoredChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // assume ansi-compatible terminal
        write!(f, "\x1b[38;5;{}m{}\x1b[0m", self.color, self.char())
    }
}

impl Display for ColoredChar {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // plaintext
        write!(f, "{}", self.char())
    }
}

impl ColoredChar {
    pub fn new(char: char, color: u32) -> Self {
        Self { char: char as u32, color }
    }
    pub fn new_rgba(char: char, r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::new(char, {
            let r = (r as u32) << 24;
            let g = (g as u32) << 16;
            let b = (b as u32) << 8;
            let a = a as u32;
            r | g | b | a
        })
    }
    pub fn char(&self) -> char {
        std::char::from_u32(self.char).unwrap()
    }
    pub fn color(&self) -> u32 {
        self.color
    }
    pub fn from_string(s: &str, color: u32) -> RVec<ColoredChar> {
        s.chars().map(|c| ColoredChar::new(c, color)).collect()
    }
}
