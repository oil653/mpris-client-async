use std::{collections::HashMap, fmt, time::Duration};

use zbus::{Connection, Proxy, names::OwnedBusName, proxy::SignalStream, zvariant::{OwnedValue, Value}};

mod metadata;
pub use metadata::Metadata;


#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    #[default]
    Stopped
}
impl PlaybackStatus{
    pub fn to_string(&self) -> String {
        match *self {
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Stopped => "Stopped"
        }.to_string()
    }
}
impl From<String> for PlaybackStatus {
    fn from(value: String) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playing") {
            Self::Playing
        } else if value == String::from("paused") {
            Self::Paused
        } else {
            Self::Stopped
        }
    }
}
impl From<&str> for PlaybackStatus {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playing") {
            Self::Playing
        } else if value == String::from("paused") {
            Self::Paused
        } else {
            Self::Stopped
        }
    }
}
impl fmt::Display for PlaybackStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum LoopStatus {
    #[default]
    /// The playback will stop after the end of the track
    None,
    /// The current track will repeat forever
    Track,
    /// The whole playlist will be repeated
    Playlist
}
impl LoopStatus{
    pub fn to_string(&self) -> String {
        match *self {
            LoopStatus::None => "None",
            LoopStatus::Track => "Track",
            LoopStatus::Playlist => "Playlist"
        }.to_string()
    }
}
impl From<String> for LoopStatus {
    fn from(value: String) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playlist") {
            Self::Playlist
        } else if value == String::from("track") {
            Self::Track
        } else {
            Self::None
        }
    }
}
impl From<&str> for LoopStatus {
    fn from(value: &str) -> Self {
        let value = value.to_lowercase();
        if value == String::from("playlist") {
            Self::Playlist
        } else if value == String::from("track") {
            Self::Track
        } else {
            Self::None
        }
    }
}
impl fmt::Display for LoopStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}


/// Something that plays media.
#[derive(Debug, Clone)]
pub struct Player {
    /// Well known name
    name: OwnedBusName,
    /// Connection to the bus
    connection: Connection,
    /// If the playback can be controlled 
    pub can_control: bool
}
impl Player {
    /// Creates a player instance from a "well known name"
    pub async fn new(name: OwnedBusName, connection: Connection) -> Self {
        let mut player = Self {
            name,
            connection,
            can_control: false
        };

        player.can_control = player.can_control().await;

        player
    }

    /// Returns the ["uique name"](https://z-galaxy.github.io/zbus/concepts.html#bus-name--service-name) of the player.
    /// <br><br>For example `org.mpris.MediaPlayer2.vlc`
    pub fn dbus_name(&self) -> String {
        self.name.to_string()
    }

    async fn get_prop<T>(&self, prop_name: &str, iface: &str) -> Result<Option<T>, zbus::Error>
    where 
        std::option::Option<T>: From<OwnedValue>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.to_owned()).await?;

        proxy.get_property(prop_name).await
    }

    async fn set_prop<'a, T>(&self, prop_name: &str, new_value: T, iface: &str) -> Result<(), zbus::Error> 
    where
        T: 'a + Into<Value<'a>>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.to_owned()).await?;

        proxy.set_property(prop_name, new_value).await.map_err(|e| e.into())
    }

    async fn call_method<A, R>(&self, method_name: &str, arguments: A, iface: &str) -> Result<R, zbus::Error> 
    where 
        A: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.to_owned()).await?;

        proxy.call(method_name, &arguments).await
    }

    pub async fn get_stream<'a>(&'a self, iface: &str) -> Result<SignalStream<'static>, zbus::Error> {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.to_owned()).await?;

        proxy.receive_all_signals().await
    }

    // =============================================================================
    // ============================     MediaPlayer2    ============================
    // =============================================================================

    //                             ====================
    //                             ===  PROPERTIES  ===
    //                             ====================

    /// The "display name" of the player. For example "Mozilla Firefox" or "VLC media player"
    pub async fn get_identity(&self) -> Result<String, zbus::Error> {
        Ok(match self.get_prop("Identity", "org.mpris.MediaPlayer2").await? {
            Some(id) => id.to_string(),
            None => String::new()
        })
    }

    /// The desktop entry of the player. For example "firefox" or "vlc"
    pub async fn get_desktop_entry(&self) -> Result<String, zbus::Error> {
        Ok(match self.get_prop("DesktopEntry", "org.mpris.MediaPlayer2").await? {
            Some(entry) => entry.to_string(),
            None => String::new()
        })
    }

    /// If quit() will work
    pub async fn can_quit(&self) -> bool {
        match self.get_prop("CanQuit", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(can_quit) => if let Ok(v) = can_quit.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// If set_fullscreen() will work
    pub async fn can_set_fullscreen(&self) -> bool {
        match self.get_prop("CanSetFullscreen", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(can_quit) => if let Ok(v) = can_quit.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// Sets the fullscreen value.
    /// <br>Note: the media player fail to set itself on fullscreen, in that case it fails silently (according to [specs](https://specifications.freedesktop.org/mpris/latest/Media_Player.html#Property:Fullscreen))
    pub async fn set_fullscreen(&self, new_state: bool) -> Result<(), zbus::Error> {
        self.set_prop::<bool>("Fullscreen", new_state.into(), "org.mpris.MediaPlayer2").await
    }

    /// If raise() will work. 
    /// <br>Note: raising is the process of bringing the media player to front, for example maximizing it, or jumping to the window in the WM / DE
    pub async fn can_raise(&self) -> bool {
        match self.get_prop("CanRaise", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(can_quit) => if let Ok(v) = can_quit.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// If the player has a tracklist 
    pub async fn has_track_list(&self) -> bool {
        match self.get_prop("HasTrackList", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(can_quit) => if let Ok(v) = can_quit.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// Returns the supported URI schemes. For example ["file", "http", "https", "rtsp"]
    pub async fn supported_uri(&self) -> Vec<String> {
        match self.get_prop("SupportedUriSchemes", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(supported_uris) => if let Ok(uris) = Vec::<String>::try_from(supported_uris) {uris} else {Vec::new()},
            None => Vec::new()
        }
    }

    /// The supported [MIME](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/MIME_types/Common_types). 
    /// They should be in the standard format, for example: audio/mpeg or application/ogg, but some players dont follow that
    pub async fn supported_mime_types(&self) -> Vec<String> {
        match self.get_prop("SupportedMimeTypes", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(mimes) => if let Ok(mimes) = Vec::<String>::try_from(mimes) {mimes} else {Vec::new()},
            None => Vec::new()
        }
    }

    //                             ====================
    //                             ===    METHODS   ===
    //                             ====================

    /// The player will try to quit, which may or may not suceed.
    pub async fn quit(&self) -> Result<(), zbus::Error> {
        self.call_method("Quit",[()], "org.mpris.MediaPlayer2").await
    }

    /// When raised, the player will try to bring itself to the front of the UI.
    pub async fn raise(&self) -> Result<(), zbus::Error> {
        self.call_method("Raise",[()], "org.mpris.MediaPlayer2").await
    }


    // =============================================================================
    // ======================    MediaPlayer2.Player    ============================
    // =============================================================================

    //                             ====================
    //                             ===  PROPERTIES  ===
    //                             ====================

    /// Get the current [playback status](PlaybackStatus)
    pub async fn get_playback_status(&self) -> PlaybackStatus {
        match self.get_prop("PlaybackStatus", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<String>() {v.into()} else {PlaybackStatus::default()},
            None => PlaybackStatus::default()
        }
    }

    /// Get the current [loop status](LoopStatus).
    pub async fn get_loop_status(&self) -> LoopStatus {
        match self.get_prop("LoopStatus", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<String>() {v.into()} else {LoopStatus::default()},
            None => LoopStatus::default()
        }
    }

    /// The rate of the playback
    pub async fn get_rate(&self) -> f64 {
        match self.get_prop("Rate", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<f64>() {v} else {1.0},
            None => 1.0
        }
    }

    /// The min rate of the playback. If not provided the player probably doesnt support setting the rate
    pub async fn get_min_rate(&self) -> Option<f64> {
        match self.get_prop("MinimumRate", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<f64>() {Some(v)} else {None},
            None => None
        }
    }
    
    /// The max rate of the playback. If not provided the player probably doesnt support setting the rate
    pub async fn get_max_rate(&self) -> Option<f64> {
        match self.get_prop("MaximumRate", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<f64>() {Some(v)} else {None},
            None => None
        }
    }

    /// If the tracks are shuffled
    pub async fn get_shuffle(&self) -> bool {
        match self.get_prop("Shuffle", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// The volume of the player. It should be between 0.0 and 1.0
    pub async fn get_volume(&self) -> Option<f64> {
        match self.get_prop("Volume", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<f64>() {Some(v)} else {None},
            None => None
        }
    }

    /// Returns how much time has passed since the start of the track.
    /// <br>You shouldnt use this, but TODO: UNIMPLEMENTED to track the position, as using this means actively polling the value.
    pub async fn get_position(&self) -> Duration {
        match self.get_prop("Rate", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<u64>() {Duration::from_micros(v)} else {Duration::from_secs(0)},
            None => Duration::from_secs(0)
        }
    }



    pub async fn can_go_next(&self) -> bool {
        match self.get_prop("CanGoNext", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    pub async fn can_go_previous(&self) -> bool {
        match self.get_prop("CanGoPrevious", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    pub async fn can_play(&self) -> bool {
        match self.get_prop("CanPlay", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    pub async fn can_pause(&self) -> bool {
        match self.get_prop("CanPause", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// Seeking is changing the media player's current position
    pub async fn can_seek(&self) -> bool {
        match self.get_prop("CanSeek", "org.mpris.MediaPlayer2.Player").await.unwrap_or(None) {
            Some(status) => if let Ok(v) = status.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }

    /// Get the metadata of the currently playing track
    pub async fn get_metadata(&self) -> Result<Metadata, zbus::Error> {
        match self.get_prop("Metadata", "org.mpris.MediaPlayer2.Player").await? {
            Some(status) => {
                let map = HashMap::<String, OwnedValue>::try_from(status)?;
                Ok(map.into())
            },
            None => Err(zbus::Error::MissingField)
        }
    }
    

    //              ========= can_control DEPENDANT =========
    pub async fn can_control(&self) -> bool {
        match self.get_prop("CanControl", "org.mpris.MediaPlayer2").await.unwrap_or(None) {
            Some(can_quit) => if let Ok(v) = can_quit.downcast_ref::<bool>() {v} else {false},
            None => false
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_status_conversion() {
        assert_eq!(PlaybackStatus::Playing, PlaybackStatus::from("Playing"));
        assert_eq!(PlaybackStatus::Paused, PlaybackStatus::from("Paused"));
        assert_eq!(PlaybackStatus::Stopped, PlaybackStatus::from("Stopped"));
    }

    #[test]
    fn loop_status_converion() {
        assert_eq!(LoopStatus::Playlist, LoopStatus::from("Playlist"));
        assert_eq!(LoopStatus::None, LoopStatus::from("None"));
        assert_eq!(LoopStatus::Track, LoopStatus::from("Track"));
    }
}