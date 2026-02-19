use std::sync::Arc;

use futures::future::join_all;

use zbus::Connection;

use crate::NormalPlayer;

#[derive(Debug, Clone)]
pub struct Mpris {
    connection: Connection,
}

impl Mpris {
    pub async fn new() -> Result<Self, zbus::Error> {
        let connection = Connection::session().await?;

        Ok(
            Self {
                connection: connection
            }
        )
    }

    pub async fn get_players(&self) -> Result<Vec<Arc<NormalPlayer>>, zbus::Error> {
        let proxy = zbus::fdo::DBusProxy::new(&self.connection).await?;
        let names = proxy.list_names().await?;

        Ok(
            join_all(names   
                .iter()
                .filter(|name| name.starts_with("org.mpris.MediaPlayer2"))
                .map (async |name| Arc::new(NormalPlayer::new(name.clone(), self.connection.clone()).await))
            ).await
        )
    }
}