use std::time::Duration;

use zbus::{Connection, Proxy, fdo, names::OwnedBusName, zvariant::{OwnedValue, Value}};

mod metadata;
pub use metadata::Metadata;

pub use crate::player::properties::{WritableProperty, Property, ControlWritableProperty};

pub mod properties;

mod enums;
pub use enums::*;



/// A player that plays something, or not, who knowns...
#[derive(Debug, Clone)]
pub struct Player {
    /// Well known name
    name: OwnedBusName,
    /// Connection to the bus
    connection: Connection
}
impl Player {
    /// Creates an instance from a "well known name", and a connection
    pub async fn new(name: OwnedBusName, connection: Connection) -> Self {
        Self {
            name,
            connection,
        }
    }

    /// Returns the ["unique name"](https://z-galaxy.github.io/zbus/concepts.html#bus-name--service-name) of the player.
    /// <br><br>For example `org.mpris.MediaPlayer2.vlc`
    pub fn dbus_name(&self) -> String {
        self.name.to_string()
    }

    /// Parses a property from the player. See [properties] for more
    pub async fn get<P>(&self, property: P) -> Result<P::Output, zbus::Error>
    where 
        P: Property,
        P::ParseAs: TryFrom<OwnedValue>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", property.interface()).await?;

        let value: OwnedValue = proxy.get_property(property.name()).await?;

        // Create the intermediate type
        let parsed: P::ParseAs = value
            .try_into()
            .map_err(|_e| zbus::Error::Variant(zbus::zvariant::Error::IncorrectType))?;

        Ok(property.into_output(parsed))
    }

    /// Set a property that implements [WritableProperty].
    pub async fn set<'a, P>(&self, property: P, new_value: P::Output) -> Result<(), fdo::Error>
    where 
        P: WritableProperty,
        P::ParseAs: 'a + Into<Value<'a>>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", property.interface()).await?;
        let transformed_value: P::ParseAs = property.from_output(new_value);

        proxy.set_property(property.name(), transformed_value).await.map(|_| ())
    }


    /// Sets a property that requires the player to allow controlling, thus [properties::CanControl] must be true. 
    pub async fn set_controlled<'a, P>(&self, property: P, new_value: P::Output) -> Result<(), fdo::Error>
    where 
        P: ControlWritableProperty,
        P::ParseAs: 'a + Into<Value<'a>>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", property.interface()).await?;
        let transformed_value: P::ParseAs = property.from_output(new_value);

        proxy.set_property(property.name(), transformed_value).await.map(|_| ())
    }

    //                             ====================
    //                             ===    METHODS   ===
    //                             ====================
    async fn call_method<A, R>(&self, method_name: &str, arguments: A, iface: Interface) -> Result<R, zbus::Error> 
    where 
        A: serde::Serialize + zbus::zvariant::DynamicType,
        R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.as_str()).await?;

        proxy.call(method_name, &arguments).await
    }

    

    /// Skips to the next track in the tracklist. If there is no next track (and endless playback and track repeat are both off), stop playback.
    /// <br>If playback is paused or stopped, it remains that way.
    /// <br>If [properties::CanGoNext] is false, attempting to call this method should have no effect.
    pub async fn next(&self) -> Result<(), zbus::Error> {
        self.call_method("Next", [()], Interface::Player).await
    }

    /// Skips to the previous track in the tracklist. If there is no previous track (and endless playback and track repeat are both off), stop playback.
    /// <br>If playback is paused or stopped, it remains that way.
    /// <br>If [properties::CanGoPrevious] is false, attempting to call this method should have no effect.
    pub async fn previous(&self) -> Result<(), zbus::Error> {
        self.call_method("Previous", [()], Interface::Player).await
    }

    /// Pauses the playback. 
    /// If [properties::CanPause] is false, this should have no effect.
    pub async fn pause(&self) -> Result<(), zbus::Error> {
        self.call_method("Pause", [()], Interface::Player).await
    }

    /// Starts or resumes the playback. 
    /// <br>If playback is already running or if [properties::CanPlay] is false, this should have no effect.
    pub async fn play(&self) -> Result<(), zbus::Error> {
        self.call_method("Play", [()], Interface::Player).await
    }

    /// Toggles the playback status between play and pause.
    /// <br>If [properties::CanPause] is false, this should have no effect, and may return with an error.
    pub async fn play_pause(&self) -> Result<(), zbus::Error> {
        self.call_method("PlayPause", [()], Interface::Player).await
    }

    /// Stops playback. Calling [Self::play] after this should restart the playlist.
    /// <br>If [properties::CanControl] is false, calling this should have no effect, and may raise an error.
    pub async fn stop(&self) -> Result<(), zbus::Error> {
        self.call_method("Stop", [()], Interface::Player).await
    }

    /// A duration to seek forward, or of backwards is true backwards. 
    /// <br>May only be used if [properties::CanSeek] is true.
    pub async fn seek(&self, duration: Duration, backwards: bool) -> Result<(), zbus::Error> {
        let modified_time = duration.as_micros() as f64 * {if backwards {-1.0} else {1.0}};
        self.call_method("Seek", [modified_time], Interface::Player).await
    }

    /// Sets the position of the track between 0 and the [length of the track](metadata::Metadata::length). track_id can be retreived from the [metadata](metadata::Metadata::trackid), but it may <b>NOT<\b> be "/org/mpris/MediaPlayer2/TrackList/NoTrack".
    /// <br>If position is greater than the [length of the track](metadata::Metadata::length), this shouldn't do anything. 
    /// <br>If [properties::CanSeek] is false this should have no effect.
    pub async fn set_position(&self, track_id: String, position: Duration) -> Result<(), zbus::Error> {
        self.call_method("SetPosition", [track_id, position.as_micros().to_string()], Interface::Player).await
    }

    /// Opens a URI, which's scheme should be an element of [properties::SupportedURIs] and the mime-type should match one of the elements of [properties::SupportedMIMEs]. 
    /// If not supported it should raise an error.
    /// <br>If the playback is stopped, it should be started. It also shouldnt be assumed the player opens the URI as soon as called!
    pub async fn open_uri(&self, uri: String) -> Result<(), zbus::Error> {
        self.call_method("OpenUri", [uri], Interface::Player).await
    }
}



// async fn call_method<A, R>(&self, method_name: &str, arguments: A, iface: &str) -> Result<R, zbus::Error> 
// where 
//     A: serde::Serialize + zbus::zvariant::DynamicType,
//     R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
// {
//     let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", iface.to_owned()).await?;

//     proxy.call(method_name, &arguments).await
// }

    // /// Returns a stream that fires every time a property of some kind had been changed. 
    // /// <br>If the connection is not active (the MPRIS object is dropped, or for some reason the underlying connection breaks it will yield None)
    // pub async fn property_changed<'a, T>(&'a self, iface: &str, prop_name: &'static str) -> Result<PropertyStream<'static, T>, zbus::Error> {
    //     let proxy: Proxy<'_> = proxy::Builder::
    //         new(&self.connection)
    //         .destination(self.name.to_owned())?
    //         .path("/org/mpris/MediaPlayer2")?
    //         .interface(iface.to_owned())?
    //         .cache_properties(proxy::CacheProperties::Yes)
    //         .build()
    //         .await?;

    //     Ok(proxy.receive_property_changed(prop_name).await)
    // }

    
    
    
    //                             ====================
    //                             ===    METHODS   ===
    //                             ====================

    // /// The player will try to quit, which may or may not succeed.
    // pub async fn quit(&self) -> Result<(), zbus::Error> {
    //     self.call_method("Quit",[()], "org.mpris.MediaPlayer2").await
    // }

    // /// When raised, the player will try to bring itself to the front of the UI.
    // pub async fn raise(&self) -> Result<(), zbus::Error> {
    //     self.call_method("Raise",[()], "org.mpris.MediaPlayer2").await
    // }


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_status_conversion() {
        assert_eq!(Playback::Playing, Playback::from("Playing"));
        assert_eq!(Playback::Paused, Playback::from("Paused"));
        assert_eq!(Playback::Stopped, Playback::from("Stopped"));
    }

    #[test]
    fn loop_status_conversion() {
        assert_eq!(Loop::Playlist, Loop::from("Playlist"));
        assert_eq!(Loop::None, Loop::from("None"));
        assert_eq!(Loop::Track, Loop::from("Track"));
    }
}