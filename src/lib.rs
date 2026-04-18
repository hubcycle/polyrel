#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod client;
pub mod ctf;
pub mod erc1155;
pub mod erc20;
pub mod neg_risk;
pub mod polymarket;
pub mod safe;

mod call;
mod error;

pub use call::{Call, NonEmptyCalls};
pub use error::PolyrelError;
