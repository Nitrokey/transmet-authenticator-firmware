#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
extern crate delog;
generate_macros!();

pub use iso7816;

pub mod command {
    pub const SIZE: usize = 7609;
    pub type Data = iso7816::Data<SIZE>;
}

pub mod response {
    pub const SIZE: usize = 7609;
    pub type Data = iso7816::Data<SIZE>;
}

// What apps can expect to send and recieve.
pub type Command = iso7816::Command<{command::SIZE}>;
pub type Response = iso7816::Response<{response::SIZE}>;

pub mod app;
pub use app::App;
pub mod dispatch;
pub mod interchanges;
