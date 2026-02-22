use std::{ops::Deref, pin::Pin, task::{Context, Poll}, time::Duration};

use futures::Stream;
use pin_project::pin_project;
use tokio::time::{Instant, Sleep};
use zbus::{proxy::{PropertyStream, SignalStream}, zvariant::{OwnedValue, Value}};

use crate::{Playback, player::Property, properties::{PlaybackStatus, Rate}, signals::Signal};


/// Returns the current position of the media of a [`Player`] every second, without polling the player.
/// <br><br>Note: this doesn't take into account the length of the media, as it might not be provided (meaning the returned position could be longer than the length of the media).
/// It only considers the current [playback status](Playback), the current [rate](properties::Rate), and if the Seeked signal was emmited, or the media changed
// TODO_DOCS
#[pin_project]
pub struct PositionStream<'a> {
    #[pin]
    playback_stream: ParsedPropertyStream<'a, PlaybackStatus>,
    
    #[pin]
    rate_stream: ParsedPropertyStream<'a, Rate>,

    #[pin]
    seeked_stream: SignalStream<'a>,

    #[pin]
    sleep: Sleep,
    // Track the last time the stream to avoid drift off the actual time (as sleep may not wake after EXACTLY 1 second)
    last_tick: Instant,
    
    rate: f64,
    playback: Playback,
    position: Duration
}
// impl<'a> Stream for PositionStream<'a> {
//     type Item = Duration;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
//         use Poll::*;
//         let mut this = self.project();

//         // See if the playback status changed
//         match this.playback_stream.as_mut().poll_next(cx) {
//             // Playback state did not change
//             Pending => {},
//             // playback_stream finished, meaning this stream should finish too
//             Ready(None) => return Ready(None),
//             Ready(Some(new_playback)) => *this.playback = new_playback
//         };

//         // Check if the rate changed
//         match this.rate_stream.as_mut().poll_next(cx) {
//             // Nothing changed
//             Pending => {},
//             Ready(None) => return Ready(None),
//             Ready(Some(new_rate)) => *this.rate = new_rate
//         }

//         match this.seeked_stream.as_mut().poll_next(cx) {
//             Pending => {},
//             Ready(None) => return Ready(None),
//             Ready(Some(new))
//         }

//         Pending
//     }
// }


#[pin_project]
/// A [`PropertyStream`], but the raw data is parsed into the corresponding [`Property`](super::properties::Property) type
pub struct ParsedPropertyStream<'a, P>
where 
    P: Property + Unpin + 'static,
    P::ParseAs: TryFrom<OwnedValue>
{
    #[pin]
    raw_stream: PropertyStream<'a, P>,
    #[pin]
    pending: Option<Pin<Box<dyn Future<Output = Result<P::ParseAs, zbus::Error>> + 'a >>>,

    p: P
}
impl<'a, P> ParsedPropertyStream<'a, P>
where
    P: Property + Unpin + 'static,
    P::ParseAs: TryFrom<OwnedValue>
{
    pub fn new(property: P, prop_stream: PropertyStream<'a, P>) -> Self {
        Self { 
            raw_stream: prop_stream, 
            pending: None, 
            p: property
        }
    }
}
impl<'a, P> Stream for ParsedPropertyStream<'a, P> 
where 
    P: Property + Unpin + 'static,
    P::ParseAs: TryFrom<OwnedValue>
{
    type Item = P::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> 
    where 
        P::ParseAs: TryFrom<OwnedValue>,
        P::Output: Send + 'static
    {
        use Poll::*;
        let mut this = self.project();

        if let Some(fut) = this.pending.as_mut().as_pin_mut() {
            match fut.poll(cx) {
                Pending => return Pending,
                Ready(Ok(result)) => {
                    let parsed: P::Output = this.p.into_output(result);
                    return Ready(Some(parsed))
                },
                Ready(Err(_e)) => {
                    return Ready(None)
                }
            }
        }

        // Try to poll the raw stream, if something is changed, start polling what changed.
        match this.raw_stream.as_mut().poll_next(cx) {
            Pending => Pending,
            Ready(None) => Ready(None),  // The raw stream is finished, meaning this stream should finish too
            Ready(Some(value)) => {
                // If something has changed, create a future that can be polled, to get what changed, and return Pending
                let fut: Pin<Box<dyn Future<Output = Result<P::ParseAs, zbus::Error>>>> = Box::pin(async move {
                    // It is safe to unwrap, as it could only fail on UNIX platforms, if Value::Fd is being parsed
                    let value: OwnedValue = value.get_raw().await?.deref().clone().try_into_owned().unwrap();
                    let converted: P::ParseAs = value.try_into().map_err(|_e| zbus::Error::Variant(zbus::zvariant::Error::IncorrectType))?;
                    Ok(converted)
                });
                *this.pending = Some(fut);
                Pending
            }
        }
    }
}



#[pin_project]
/// A [`SignalStream`], but the raw data is parsed into the corresponding [`Signal`](super::signals::Signal) type
pub struct ParsedSignalStream<'a, S>
where 
    S: Signal + Unpin + 'static,
    S::ParseAs: TryFrom<OwnedValue>
{
    #[pin]
    raw_stream: SignalStream<'a>,

    s: S
}
impl<'a, S> ParsedSignalStream<'a, S>
where
    S: Signal + Unpin + 'static,
    S::ParseAs: TryFrom<OwnedValue>
{
    pub fn new(signal: S, signal_stream: SignalStream<'a>) -> Self {
        Self { 
            raw_stream: signal_stream,
            s: signal
        }
    }
}
// impl<'a, S> Stream for ParsedSignalStream<'a, S> 
// where 
//     S: Signal + Unpin + 'static,
//     S::ParseAs: TryFrom<OwnedValue>
// {
//     type Item = S::Output;

//     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> 
//     where 
//         S::ParseAs: TryFrom<OwnedValue>,
//         S::Output: Send + 'static
//     {
//         use Poll::*;
//         let mut this = self.project();

//         // Try to poll the raw stream, if something is changed, start polling what changed.
//         match this.raw_stream.as_mut().poll_next(cx) {
//             Pending => Pending,
//             Ready(None) => Ready(None),  // The raw stream is finished, meaning this stream should finish too
//             Ready(Some(value)) => {
//                 let body = value.body();
//                 let deser = body.deserialize::<S::ParseAs>();
//                 let value: S::ParseAs = match deser {
//                     Ok(value) => value,
//                     Err(_e) => return Ready(None)
//                 };

//                 Ready(Some(this.s.into_output(value)))
                    
                    
//                 // let converted: S::ParseAs = value.try_into().map_err(|_e| zbus::Error::Variant(zbus::zvariant::Error::IncorrectType))?;
//             }
//         }
//     }
// }