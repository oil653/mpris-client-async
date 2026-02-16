mod player;
pub use player::{ Player, Metadata, LoopStatus, PlaybackStatus };

/// Provides the basics, like the conncetion to dbus
mod mpris;
pub use mpris::Mpris;