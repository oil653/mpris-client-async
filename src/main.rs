use std::{collections::HashMap, time::Duration};

use futures::{StreamExt, stream::select_all};
use mpris_client_async::{Loop, Mpris, properties::*, signals::{SEEKED, Seeked, Signal}};
use zbus::zvariant::OwnedValue;

#[tokio::main]
async fn main() {
    let mpris: Mpris = Mpris::new().await.unwrap();
    let players = mpris.get_players().await.unwrap();
    
    let mut metadata_streams= Vec::new();
    let mut seeked_signals = Vec::new();

    // Get the "unique name" of the players
    for player in &players {
        println!("Player: {}, with identity {} with desktop entry {}", 
            player.dbus_name(),
            player.get(Identity).await.unwrap_or("??".to_string()),
            player.get(DesktopEntry).await.unwrap_or("??".to_string())
        );

        // println!("Player identity: {}", player.get::<properties::Identity>().await.unwrap_or("Freak"));

        println!("\tMediaPlayer2:");
        println!("\t\tCapabilities:");
        println!("\t\t\tcan quit? {}",                  player.get(CanQuit).await.unwrap_or(false));
        println!("\t\t\tcan set fullscreen? {}",        player.get(CanSetFullscreen).await.unwrap_or(false));
        println!("\t\t\tcan raise? {}",                 player.get(CanRaise).await.unwrap_or(false));
        println!("\t\t\thas track list? {}",            player.get(HasTrackList).await.unwrap_or(false));
        println!("\t\t\tsupported URI: {:?}",           player.get(SupportedURIs).await.unwrap_or(vec![]));
        println!("\t\t\tsupported MIME types: {:?}",    player.get(SupportedMIMEs).await.unwrap_or(vec![]));

        // if player.get(CanSetFullscreen).await.unwrap_or(false) {
        //     player.set(Fullscreen, !player.get(Fullscreen).await.unwrap_or(false)).await.expect("Failed to set fullscreen.");
        // }

        println!("\tMediaPlayer2.Player:");
        println!("\t\tPlaybackStatus: {}",              player.get(PlaybackStatus).await.unwrap_or(mpris_client_async::Playback::Stopped));
        println!("\t\tLoopStatus: {}",                  player.get(LoopStatus).await.unwrap_or(mpris_client_async::Loop::None));
        println!("\t\trate: {}",                        player.get(Rate).await.unwrap_or(1.0));
        println!("\t\tmax rate: {:?}, min rate: {:?}",  player.get(MinimumRate).await, player.get(MaximumRate).await);
        println!("\t\tis shuffled: {}",                 player.get(Shuffle).await.unwrap_or(false));
        println!("\t\tvolume: {}",                      player.get(Volume).await.unwrap_or(0.0));
        println!("\t\tPosition (in secs): {}",          player.get(Position).await.unwrap_or(Duration::from_secs(0)).as_secs());
        println!("\t\tcan_seek: {}",                    player.get(CanSeek).await.unwrap_or(false));
        println!("\t\tcan_control: {}",                 player.get(CanControl).await.unwrap_or(false));

        // println!("\t\t\tMetadata: {:#?}",               player.get(Metadata).await);

        // let can_control = player.get(CanControl).await.unwrap_or(false);
        // if can_control {
        //     let result = player.set_controlled(LoopStatus, Loop::Track).await;
        //     println!("loop status set result: {:?}", result);
        // }
        // let min_rate = player.get(MinimumRate).await;
        // if let Ok(_) = min_rate && can_control {
        //     let result = player.set_controlled(Rate, 0.5).await;
        //     println!("set rate result: {:?}", result);
        // }
        // let path_to_media = String::from("file:///home/USER/definitely_not_hentai.mp3");
        // if can_control && player.get(SupportedURIs).await.unwrap_or(vec![]).contains(&String::from("file")) {
        //     player.open_uri(path_to_media).await.expect("Failed to open peak");
        // }

        
        // Subscribe to the event when Metadata changed.
        // let cigany: mpris_client_async::streams::ParsedPropertyStream<'_, Rate> = player.property_changed_stream(Metadata).await.unwrap();
        metadata_streams.push(player.property_changed_stream(PlaybackStatus).await.unwrap());
        seeked_signals.push(player.subscribe(Seeked).await.unwrap());

        println!();
    }

    // Combine the streams of the changes. YOU CANNOT KNOW WHICH PLAYER A MESSAGE IS FROM!
    let mut combined = select_all(metadata_streams);
    while let Some(mtd) = combined.next().await {
        println!("Metadata changed for some player: {:#?}", mtd);
    }


    
    // Listen for seeked signals (wont know which player it is from, as they're combined)
    // let mut combined = select_all(seeked_signals);
    // while let Some(seeked_to) = combined.next().await {
    //     println!("Some player seeked to: {}", seeked_to.as_secs());
    // }
}