//! This crate allows one-liners to read from a terminal
//! A minimal example to get an i32 from the command line is
//!
//! ```rust,no_run
//! #[macro_use] extern crate text_io;
//! fn main() {
//!     let i: i32 = read!();
//! }
//! ```
//!
//! The `read!()` macro will always read until the next ascii whitespace character
//! (`\n`, `\r`, `\t` or space).
//!
//! Any type that implements the `FromStr` trait can be read with the `read!` macro.
//!
//! # Advanced
//! Text parsing can be done similar to `println!` by adding a format string
//! to the macro:
//!
//! ```rust,no_run
//! # #[macro_use]
//! # extern crate text_io;
//! # fn main() {
//! let i: i32 = read!("The answer: {}!");
//! # }
//! ```
//!
//! This will read `"The answer: "`, then an integer, then an exclamation mark. Any deviation from
//! the format string will result in a panic.
//!
//! Note: only a single value can be read per `read!` invocation.

use std::error;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum Error {
    MissingMatch,
    MissingClosingBrace,
    UnexpectedValue(u8, Option<u8>),
    InvalidUtf8(Vec<u8>),
    PartialUtf8(usize, Vec<u8>),
    Parse(String, &'static str),
    #[doc(hidden)]
    __NonExhaustive__,
}

impl error::Error for Error {
    fn description(&self) -> &str {
        use crate::Error::*;

        match *self {
            MissingMatch => "Bad read! format string: did not contain {{}}",
            MissingClosingBrace => "found single open curly brace at the end of the format string",
            UnexpectedValue(..) => "found value not matching the pattern",
            InvalidUtf8(..) => "input was not valid utf8",
            PartialUtf8(..) => "input was only partially valid utf8",
            Parse(..) => "could not parse input as target type",
            __NonExhaustive__ => unreachable!(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use crate::Error::*;
        use std::str::from_utf8;

        match *self {
            InvalidUtf8(ref raw) => write!(f, "input was not valid utf8: {:?}", raw),
            Parse(ref s, arg) => write!(f, "could not parse {} as target type of {}", s, arg),
            UnexpectedValue(exp, act) => write!(
                f,
                "found value {:?} not matching the pattern value {}",
                act.map(|b| b as char),
                exp as char
            ),
            PartialUtf8(n, ref raw) => write!(
                f,
                "input was only partially valid utf8: \"{}\" followed by {:?}",
                from_utf8(&raw[..n]).unwrap(),
                &raw[n..]
            ),
            _ => write!(f, "{}", <Error as error::Error>::description(self)),
        }
    }
}

pub fn match_next(expected: u8, iter: &mut dyn Iterator<Item = u8>) -> Result<(), Error> {
    let next = iter.next();
    if next != Some(expected) {
        return Err(Error::UnexpectedValue(expected, next))?;
    }
    Ok(())
}

pub fn parse_capture<T>(
    name: &'static str,
    next: Option<u8>,
    iter: &mut dyn Iterator<Item = u8>,
) -> Result<T, Error>
where
    T: FromStr,
    <T as FromStr>::Err: ::std::fmt::Debug,
{
    static WHITESPACES: &'static [u8] = b"\t\r\n ";
    let raw: Vec<u8> = match next {
        Some(c) => iter.take_while(|&ch| ch != c).collect(),
        None => iter
            .skip_while(|ch| WHITESPACES.contains(ch))
            .take_while(|ch| !WHITESPACES.contains(ch))
            .collect(),
    };
    match String::from_utf8(raw) {
        Ok(s) => FromStr::from_str(&s).map_err(|_| Error::Parse(s, name)),
        Err(e) => {
            let n = e.utf8_error().valid_up_to();
            let raw = e.into_bytes();
            if n == 0 {
                Err(Error::InvalidUtf8(raw))
            } else {
                Err(Error::PartialUtf8(n, raw))
            }
        }
    }
}

/// ```rust,no_run
/// # #[macro_use]
/// # extern crate text_io;
/// # fn main() {
/// let i: i32 = try_read!("The answer: {}!").unwrap();
/// let i: Result<i32, _> = try_read!("The {}{{}}!", "The answer is 42!".bytes());
/// assert!(i.is_err());
/// # }
/// ```
///
/// ```rust
/// # #[macro_use]
/// # extern crate text_io;
/// # fn main() {
/// let i: Result<i32, _> = try_read!("The answer is {}!", "The answer is 42!".bytes());
/// assert!(i.is_ok());
///
/// let i: Result<i32, _> = try_read!("The {}{{}}!", "The answer is 42!".bytes());
/// assert!(i.is_err());
/// # }
/// ```
#[macro_export]
macro_rules! try_read(
    () => { try_read!("{}") };
    ($text:expr) => {{
        (|| -> Result<_, $crate::Error> {
            let __try_read_var__;
            try_scan!($text, __try_read_var__);
            Ok(__try_read_var__)
        })()
    }};
    ($text:expr, $input:expr) => {{
        (|| -> Result<_, $crate::Error> {
            let __try_read_var__;
            try_scan!($input => $text, __try_read_var__);
            Ok(__try_read_var__)
        })()
    }};
);

/// ```rust,no_run
/// # #[macro_use]
/// # extern crate text_io;
/// # use std::error::Error;
/// # fn main() {}
/// fn parser() -> Result<i32, Box<Error>> {
///     let i: i32;
///     let text = "The answer is 42!";
///
///     try_scan!(text.bytes() => "The answer is {}!", i);
///
///     assert_eq!(i, 1);
///     Ok(i)
/// }
/// ```
#[macro_export]
macro_rules! try_scan(
    ($pattern:expr, $($arg:expr),*) => {
        use ::std::io::Read;
        try_scan!(::std::io::stdin().bytes().map(std::result::Result::unwrap) => $pattern, $($arg),*);
        format_args!($pattern, $($arg),*);
    };
    ($input:expr => $pattern:expr, $($arg:expr),*) => {{
        try_scan!(@impl question_mark; $input => $pattern, $($arg),*)
    }};
    (@question_mark: $($e:tt)+) => {{
        ($($e)+)?
    }};
    (@unwrap: $($e:tt)+) => {{
        ($($e)+).unwrap()
    }};
    (@impl $action:tt; $input:expr => $pattern:expr, $($arg:expr),*) => {{
        use $crate::{Error, match_next, parse_capture};

        // typesafe macros :)
        let pattern: &'static str = $pattern;
        let stdin: &mut Iterator<Item = u8> = &mut ($input);

        let mut pattern = pattern.bytes();

        $(
            $arg = loop {
                match try_scan!(@$action: pattern.next().ok_or(Error::MissingMatch)) {
                    b'{' => match try_scan!(@$action: pattern.next().ok_or(Error::MissingClosingBrace)) {
                        b'{' => try_scan!(@$action: match_next(b'{', stdin)),
                        b'}' => break try_scan!(@$action: parse_capture(stringify!($arg), pattern.next(), stdin)),
                        _ => return try_scan!(@$action: Err(Error::MissingClosingBrace)),
                    },
                    c => try_scan!(@$action: match_next(c, stdin)),
                }
            };
        )*

        for c in pattern {
            try_scan!(@$action: match_next(c, stdin))
        }

        format_args!($pattern, $($arg),*);
    }};
);

/// All text input is handled through this macro
#[macro_export]
macro_rules! read(
    ($($arg:tt)*) => {
        try_read!($($arg)*).unwrap()
    };
);

/// This macro allows to pass several variables so multiple values can be read
#[macro_export]
macro_rules! scan(
    ($text:expr, $($arg:expr),*) => {
        use ::std::io::Read;
        scan!(::std::io::stdin().bytes().map(std::result::Result::unwrap) => $text, $($arg),*);
        format_args!($text, $($arg),*);
    };
    ($input:expr => $pattern:expr, $($arg:expr),*) => {{
        try_scan!(@impl unwrap; $input => $pattern, $($arg),*)
    }};
);
