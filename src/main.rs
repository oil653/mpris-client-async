use futures::{StreamExt, stream};
use mpris_client_async::Mpris;

#[tokio::main]
async fn main() {
    let mpris = Mpris::new().await.unwrap();
    let players = mpris.get_players().await.unwrap();
    
    let mut streams = Vec::new();

    // Get the "unique name" of the players
    for player in players {
        println!("Player: {}, with identity {} with desktop entry {}", 
            player.dbus_name(),
            player.get_identity().await.unwrap_or("??".to_string()), 
            player.get_desktop_entry().await.unwrap_or("??".to_string())
        );

        println!("\tMediaPlayer2:");
        println!("\t\tCapabilities:");
        println!("\t\t\tcan quit? {}", player.can_quit().await);
        println!("\t\t\tcan set fullscreen? {}", player.can_set_fullscreen().await);
        println!("\t\t\tcan raise? {}", player.can_raise().await);
        println!("\t\t\thas track list? {}", player.has_track_list().await);
        println!("\t\t\tsupported URI: {:?}", player.supported_uri().await);
        println!("\t\t\tsupported MIME types: {:?}", player.supported_uri().await);

        // if player.can_quit().await {
        //     _ = player.quit().await;
        // }

        // if player.can_raise().await {
        //     _ = player.raise().await;
        // }

        println!("\tMediaPlayer2.Player:");
        println!("\t\tPlaybackStatus: {}", player.get_playback_status().await);
        println!("\t\tLoopStatus: {}", player.get_loop_status().await);
        println!("\t\trate: {}", player.get_rate().await);
        println!("\t\tmax rate: {:?} min rate: {:?}", player.get_max_rate().await, player.get_min_rate().await);
        println!("\t\tis shuffled: {}", player.get_shuffle().await);
        println!("\t\tvolume: {:?}", player.get_volume().await);
        println!("\t\tPosition (in secs): {}", player.get_position().await.as_secs());
        println!("\t\tcan_seek: {}", player.can_seek().await);
        println!("\t\tcan_control: {}", player.can_control().await);

        // println!("\t\t\tMetadata: {:#?}", player.get_metadata().await);


        streams.push(player.get_stream("org.mpris.MediaPlayer2").await.unwrap());

        println!("");
    }
    
    // while let Some(msg) = streams[0].next().await {
    //     println!("{:?}", msg);
    // }
}