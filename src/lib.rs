/// A player, and related stuff
mod player;
pub use player::{ 
    Player, 
    Metadata, 
    Loop, 
    Playback, 
    properties, 
    signals, 
    streams
};

mod mpris;
pub use mpris::{ Mpris, PlayerEvent };

pub use zbus::Error;