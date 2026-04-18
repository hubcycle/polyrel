#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod ctf;
pub mod erc1155;
pub mod erc20;
pub mod neg_risk;
pub mod polymarket;

mod call;

pub use call::Call;
