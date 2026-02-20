use std::sync::Arc;

use futures::future::join_all;

use zbus::Connection;

use crate::Player;

#[derive(Debug, Clone)]
/// Provides a convenient way to connect to the dbus and retrieve the MPRIS players.
pub struct Mpris {
    connection: Connection,
}

impl Mpris {
    /// Creates a new connection
    pub async fn new() -> Result<Self, zbus::Error> {
        let connection = Connection::session().await?;

        Ok(
            Self {
                connection: connection
            }
        )
    }

    /// Gets all currently available players.
    pub async fn get_players(&self) -> Result<Vec<Arc<Player>>, zbus::Error> {
        let proxy = zbus::fdo::DBusProxy::new(&self.connection).await?;
        let names = proxy.list_names().await?;

        Ok (
            join_all(names   
                    .iter()
                    .filter(|name| name.starts_with("org.mpris.MediaPlayer2"))
                    .map (async |name| Player::new(name.clone(), self.connection.clone()).await)
                )
            .await
            .into_iter()
            .try_fold(Vec::new(), |mut vec, player| match player {
                Ok(v) => { 
                    vec.push(Arc::new(v));
                    Ok(vec)   
                },
                Err(e) => return Err(e)
            })?
        )
    }
}