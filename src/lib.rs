mod decode;
mod encode;
mod error;
mod io;
mod types;

pub use decode::{DecodeLimits, decode, decode_with};
pub use encode::{EncodeOptions, encode, encode_with};
pub use error::{Error, Result};
pub use types::{
    Angles, Appearance, Color3, Entity, Identity, Material, Scene, Script, Shape, SpawnLocation,
    Visibility,
};

pub const SIGNATURE: [u8; 4] = *b"CVRA";
pub const VERSION: u16 = 1;

pub(crate) const FLAG_COMPRESSED: u8 = 0b0000_0001;

pub(crate) const COMPONENT_IDENTITY: u32 = 0;
pub(crate) const COMPONENT_APPEARANCE: u32 = 1;

pub const COMPONENT_SCRIPT: u32 = 2;
pub const COMPONENT_SPAWN_LOCATION: u32 = 3;
pub const COMPONENT_LOCKED: u32 = 4;
pub const COMPONENT_MATERIAL: u32 = 5;
