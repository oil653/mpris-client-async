use zbus::{Connection, Proxy, fdo, names::OwnedBusName, zvariant::{OwnedValue, Value}};

mod metadata;
pub use metadata::Metadata;

pub use crate::player::properties::{WritableProperty, Property, ControlWriteProperty};

pub mod properties;

mod enums;
pub use enums::*;



/// A player that cannot be [controlled](ControlWriteProperty)
#[derive(Debug, Clone)]
pub struct NormalPlayer {
    /// Well known name
    name: OwnedBusName,
    /// Connection to the bus
    connection: Connection
}
impl NormalPlayer {
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

    /// Set a regular property. To set something that implements [ControlWriteProperty], check [Self::set_controlled]
    pub async fn set<'a, P>(&self, property: P, new_value: P::ParseAs) -> Result<(), fdo::Error>
    where 
        P: WritableProperty,
        P::ParseAs: 'a + Into<Value<'a>>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", property.interface()).await?;

        proxy.set_property(property.name(), new_value).await.map(|_| ())
    }


    /// Sets a property that requires CanControl to true. If it's false, it will return with an [fdo::Error]
    pub async fn set_controlled<'a, P>(&self, property: P, new_value: P::ParseAs) -> Result<(), fdo::Error>
    where 
        P: ControlWriteProperty,
        P::ParseAs: 'a + Into<Value<'a>>
    {
        let proxy = Proxy::new(&self.connection, self.name.to_owned(), "/org/mpris/MediaPlayer2", property.interface()).await?;

        proxy.set_property(property.name(), new_value).await.map(|_| ())
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