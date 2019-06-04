#![cfg_attr(feature = "cargo-clippy", allow(match_same_arms))]

mod error;
mod messages;
pub mod router;
mod utils;

use self::error::*;

pub use messages::{ArgDict, ArgList, CallError, Dict, InvocationPolicy, List, MatchingPolicy,
                   Reason, Value, URI};
use messages::{ErrorType, Message};
pub use router::Router;

pub type CallResult<T> = Result<T, CallError>;
pub type WampResult<T> = Result<T, Error>;
pub type ID = u64;
