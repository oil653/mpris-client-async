use std::{ops::Deref, pin::Pin, task::{Context, Poll}, time::Duration};

use futures::Stream;
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use tokio::time::{Instant, Sleep, sleep, sleep_until};
use zbus::{proxy::{PropertyStream, SignalStream}, zvariant::{OwnedValue, Type}};

use crate::{Playback, player::Property, properties::{PlaybackStatus, Rate}, signals::{Seeked, Signal}};


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
    seeked_stream: ParsedSignalStream<'a, Seeked>,

    #[pin]
    sleep: Sleep,
    // Track the last time the stream to avoid drift off the actual time (as sleep may not wake after EXACTLY 1 second)
    last_tick: Instant,
    
    rate: f64,
    playback: Playback,
    position: Duration
}
impl<'a> Stream for PositionStream<'a> {
    type Item = Duration;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        let mut this = self.project();

        // Check if the rate changed
        match this.rate_stream.as_mut().poll_next(cx) {
            // Nothing changed
            Pending => {},
            Ready(None) => return Ready(None),
            Ready(Some(new_rate)) => {
                let old_rate = *this.rate;
                *this.rate = new_rate;

                if *this.playback == Playback::Playing {
                    // How much time passsed since the last tick
                    let delta = Instant::now() - *this.last_tick;
                    let new_position = Duration::from_micros(((*this.position + delta).as_micros() as f64 * old_rate) as u64);

                    this.sleep.set(sleep_until(Instant::now() + Duration::from_secs(1)));

                    *this.last_tick = Instant::now();
                    *this.position = new_position;

                    return Ready(Some(new_position));
                }
            }
        }

        // See if the playback status changed
        match this.playback_stream.as_mut().poll_next(cx) {
            // Playback state did not change
            Pending => {},
            // playback_stream finished, meaning this stream should finish too
            Ready(None) => return Ready(None),
            Ready(Some(new_playback)) => {
                let old_playback = *this.playback;
                *this.playback = new_playback;

                this.sleep.set(sleep_until(Instant::now() + Duration::from_secs(1)));

                match (old_playback, *this.playback) {
                    (Playback::Paused | Playback::Stopped, Playback::Playing) => {
                        *this.last_tick = Instant::now();
                        return Ready(Some(*this.position))
                    },
                    (Playback::Playing, Playback::Paused | Playback::Stopped) => {
                        let delta = Instant::now() - *this.last_tick;
                        *this.position = Duration::from_micros(((*this.position + delta).as_micros() as f64 * *this.rate) as u64);
                        *this.last_tick = Instant::now();

                        return Ready(Some(*this.position));
                    },
                    _ => {}
                }
            }
        };

        match this.seeked_stream.as_mut().poll_next(cx) {
            Pending => {},
            Ready(None) => return Ready(None),
            Ready(Some(new)) => {
                *this.position = new;
                *this.last_tick = Instant::now();

                // Set next sleep cycle
                this.sleep.set(sleep_until(Instant::now() + Duration::from_secs(1)));

                return Ready(Some(new))
            }
        }

        match this.sleep.as_mut().poll(cx) {
            Pending => Pending,
            Ready(_) => {
                let delta = Instant::now() - *this.last_tick;
                let new_position = Duration::from_micros(((*this.position + delta).as_micros() as f64 * *this.rate) as u64);

                *this.position = new_position;
                *this.last_tick = Instant::now();

                this.sleep.set(sleep_until(Instant::now() + Duration::from_secs(1)));

                Ready(Some(*this.position))
            }
        }
    }
}


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
    S: Signal + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static
{
    #[pin]
    raw_stream: SignalStream<'a>,

    s: S
}
impl<'a, S> ParsedSignalStream<'a, S>
where
    S: Signal + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static
{
    pub fn new(signal: S, signal_stream: SignalStream<'a>) -> Self {
        Self { 
            raw_stream: signal_stream,
            s: signal
        }
    }
}
impl<'a, S> Stream for ParsedSignalStream<'a, S> 
where 
    S: Signal + 'static,
    S::Output: Send + 'static,
    S::ParseAs: DeserializeOwned + Send + 'static + Type
{
    type Item = S::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        use Poll::*;
        let mut this = self.project();

        // Try to poll the raw stream, if something is changed, start polling what changed.
        match this.raw_stream.as_mut().poll_next(cx) {
            Pending => Pending,
            Ready(None) => Ready(None),  // The raw stream is finished, meaning this stream should finish too
            Ready(Some(msg)) => {
                let body = msg.body();
                let parsed: S::ParseAs = match body.deserialize_unchecked() {   // Lets hope unchecked is fine
                    Ok(v) => v,
                    Err(_e) => return Ready(None)
                };

                Ready(Some(this.s.into_output(parsed)))
                }
        }
    }
}

