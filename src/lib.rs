/// A player
mod player;
pub use player::{ NormalPlayer, Metadata, Loop, Playback, properties };

/// Provides the basics, like the conncetion to dbus
mod mpris;
pub use mpris::Mpris;