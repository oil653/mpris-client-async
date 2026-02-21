/// A player, and related stuff
mod player;
pub use player::{ Player, Metadata, Loop, Playback, properties, signals };

mod mpris;
pub use mpris::Mpris;