use core::num::ParseIntError;
use std::boxed::Box;
use std::{error, fmt, io, result};

pub struct Error {
    pub desc: String,
    pub cause: Option<Box<dyn error::Error>>,
}

pub type Result<T> = result::Result<T, Error>;

macro_rules! errfmt {
    ($msg:expr) => (Err(Error::from($msg.to_string())));
    ($err:expr, $msg:expr) => (Err(Error::from(($err, $msg.to_string()))));
    ($err:expr, $fmt:expr, $($arg:tt)+) => (Err(Error::from(($err, format!($fmt, $($arg)+)))));
}

macro_rules! tryfmt {
    ($result:expr, $($arg:tt)+)  => (match $result {
            Ok(val) => val,
            Err(err) => {
                return errfmt!(err, $($arg)+)
            }
    })
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &*self.desc
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        self.cause.as_deref()
    }
}

macro_rules! from {
    ($error:ty) => {
        impl From<($error, String)> for Error {
            fn from((error, desc): ($error, String)) -> Error {
                Error {
                    desc: format!("{}: {}", desc, error),
                    cause: Some(Box::new(error)),
                }
            }
        }
    };
}

from!(io::Error);
from!(nix::Error);
from!(ParseIntError);

impl From<(Error, String)> for Error {
    fn from((error, desc): (Error, String)) -> Error {
        let desc_with_error = if desc.is_empty() {
            format!("{}", error)
        } else {
            format!("{}: {}", desc, error)
        };
        Error {
            desc: desc_with_error,
            cause: match error.cause {
                Some(cause) => Some(cause),
                None => None,
            },
        }
    }
}

impl From<String> for Error {
    fn from(desc: String) -> Error {
        Error { desc, cause: None }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self.desc)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        fmt::Display::fmt(self, f)
    }
}
