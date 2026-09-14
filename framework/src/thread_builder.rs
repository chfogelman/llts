use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;
use std::time::SystemTime;

use crate::container::Container;
use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::hub::live_hub::LiveHub;
use crate::{Context, Hub, Pipeline, Pollable, StartEvent, ThreadLocalEnvelope};

pub struct Disconnected;

pub struct ThreadHub<T>
where
    T: Thread,
{
    senders:  T::Senders,
    hub_impl: LiveHub<<T as Thread>::Pipeline>,
}

impl<T> Hub for ThreadHub<T>
where
    T: Thread,
{
    type Pipeline = T::Pipeline;

    fn poll(&self, pipeline: &mut Self::Pipeline) {
        self.hub_impl.poll(pipeline);
    }

    fn register_pollable(&self, pollable: Box<dyn Pollable<Self::Pipeline>>) {
        self.hub_impl.register_pollable(pollable);
    }

    fn defer<Msg>(&self, msg: Msg) {
        self.hub_impl.defer(msg)
    }

    fn now(&self) -> SystemTime {
        self.hub_impl.now()
    }

    fn start_timer<Event: Copy>(&self, event: Event, duration: std::time::Duration) {
        self.hub_impl.start_timer(event, duration);
    }

    fn stop(&self, reason: String) {
        self.hub_impl.stop(reason);
    }
}

impl<T> Context for Rc<ThreadHub<T>>
where
    T: Thread,
{
    type Hub = ThreadHub<T>;

    fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
        if self.senders.send(msg).is_err() {
            self.stop("Sender disconnected".into());
        }
    }

    fn get_hub(&self) -> &Self::Hub {
        self
    }
}

impl<T> Pipeline for Rc<ThreadHub<T>>
where
    T: Thread,
{
    fn dispatch<Msg>(&mut self, _msg: &Msg) {
        // Intentionally blank.
        // This is the end of the line for dispatching
        // TODO: maybe this should be used to implement input event capture and sequencing?
    }
}

pub struct ThreadImpl<C, Others>
where
    C: Container + Send + 'static,
    Others: Bus + 'static,
{
    container: C,
    receiver:  <Self as Thread>::Receiver,
    senders:   <Self as Thread>::Senders,
}

impl<C, B> ThreadImpl<C, B>
where
    C: Container + Send,
    B: Bus,
{
    fn start_event_loop(self) {
        let Self {
            container,
            receiver,
            senders,
        } = self;
        let thread_hub = Rc::new(ThreadHub::<Self> {
            senders,
            hub_impl: LiveHub::new(),
        });
        thread_hub.register_pollable(Box::new(receiver));
        let mut pipeline = container.finalize(Rc::clone(&thread_hub));

        pipeline.dispatch(&StartEvent);
        while !thread_hub.hub_impl.stop_requested() {
            thread_hub.poll(&mut pipeline);
        }
    }
}

pub trait ThreadBuilder {
    type Threads;

    fn start_threads(self) -> Vec<thread::Result<()>>;
}

pub trait Bus {
    type Handles<T: Container + Send + 'static>: Senders + Send + 'static;
}

#[allow(type_alias_bounds)]
type Channel<T: Thread> = (T::Sender, T::Receiver);

impl<P: Pipeline> Pollable<P> for Receiver<Envelope<P>> {
    fn poll(&self, target: &mut P) -> Result<(), crate::PollError> {
        let next = self.try_recv();
        match next {
            Ok(msg) => msg.dispatch_onto(target),
            Err(TryRecvError::Empty) => (),
            Err(TryRecvError::Disconnected) => target.get_hub().stop("Receiver disconnected".into()),
        }
        Ok(())
    }
}

pub trait Thread {
    type Pipeline: Pipeline;
    type Channel;
    type Sender;
    type Receiver;
    type Senders: Senders;
    fn make_channel() -> Channel<Self>;

    fn spawn<'scope, 'env: 'scope>(
        self,
        scope: &'scope thread::Scope<'scope, 'env>,
    ) -> thread::ScopedJoinHandle<'scope, ()>;
}

impl<C, B> Thread for ThreadImpl<C, B>
where
    C: Container + Send,
    B: Bus,
{
    type Pipeline = C::Finalized<Rc<ThreadHub<Self>>>;
    type Sender = Sender<Envelope<Self::Pipeline>>;
    type Receiver = Receiver<Envelope<Self::Pipeline>>;
    type Channel = Channel<Self>;
    type Senders = <B as Bus>::Handles<C>;

    fn make_channel() -> Channel<Self> {
        channel()
    }

    fn spawn<'scope, 'env: 'scope>(
        self,
        scope: &'scope thread::Scope<'scope, 'env>,
    ) -> thread::ScopedJoinHandle<'scope, ()> {
        let thread_impl = self;
        scope.spawn(move || {
            thread_impl.start_event_loop();
        })
    }
}

pub trait Senders {
    fn send<Msg>(&self, msg: impl ThreadLocalEnvelope<Msg>) -> Result<(), Disconnected>;
}

impl Senders for () {
    fn send<Msg>(&self, _msg: impl ThreadLocalEnvelope<Msg>) -> Result<(), Disconnected> {
        Ok(())
    }
}

macro_rules! peel_impl_senders_for_handles {
    ($t:ident, $($ts:ident,)*) => {
        impl_senders_for_handles!{$($ts,)*}
    }
}

macro_rules! impl_senders_for_handles {
    () => {};
    ($($ts:ident,)+) => {
        impl<$($ts: Pipeline,)+> Senders for ($(Sender<Envelope<$ts>>,)+) {
            #[allow(non_snake_case)]
            fn send<Msg>(&self, msg: impl ThreadLocalEnvelope<Msg>) -> Result<(), Disconnected> {
                let msg = msg.alloc_envelope();
                let ($($ts,)+) = self;
                let successful = [$(
                    $ts.send(msg.build_envelope()).is_err(),
                )+].into_iter().any(|b| b);
                if successful {
                    Ok(())
                } else {
                    Err(Disconnected)
                }
            }
        }
        peel_impl_senders_for_handles!{$($ts,)+}
    };
}

impl_senders_for_handles! {A, B, C, D, E, F, G, H, I, J,}

macro_rules! impl_bus_type {
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), (), ($($accum:ty,)*)) => { ($($accum,)*) };
    ($slf:ident, ($($prev_xs:ident,)*), ($x:ident, $($xs:ident,)*), (), ($($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_bus_type!($slf, ($($prev_xs,)* $x,), ($($xs,)*), (), ($($ys,)*),
            ($($accum,)* <ThreadImpl<$x, (($($prev_xs,)*), ($($xs,)* $slf, $($ys,)*))> as Thread>::Sender,))
    };
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), ($y:ident, $($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_bus_type!($slf, ($($prev_xs,)*), (), ($($prev_ys,)* $y,), ($($ys,)*),
            ($($accum,)* <ThreadImpl<$y, (($($prev_xs,)* $slf, $($prev_ys,)*), ($($ys,)*))> as Thread>::Sender,))
    };
}

macro_rules! impl_buses {
    (($($ts:ident,)*), ()) => {};
    (($($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl<$($ts: Container + Send + 'static,)* $($us: Container + Send + 'static,)*> Bus for (($($ts,)*), ($($us,)*)) {
            type Handles<$u: Container + Send + 'static> = impl_bus_type!{$u, (), ($($ts,)*), (), ($($us,)*), ()};
        }
        impl_buses!{($($ts,)* $u,), ($($us,)*)}
    };
}

macro_rules! impl_all_buses {
    () => {};
    ($t:ident, $($ts:ident,)*) => {
        impl_buses!{(), ($t, $($ts,)*)}
        impl_all_buses!{$($ts,)*}
    };
}

impl_all_buses! {A, B, C, D, E, F, G, H, I, J,}

macro_rules! thread_builder_threads_type {
    ( ($($prev:ident,)*), (), ($($accum:ty,)*) ) => {
        ($($accum,)*)
    };
    ( ($($prev:ident,)*), ($t:ident, $($ts:ident,)*), ($($accum:ty,)*) ) => {
        thread_builder_threads_type!(($($prev,)* $t,), ($($ts,)*), ($($accum,)* ThreadImpl<$t, (($($prev,)*), ($($ts,)*))>,))
    };
}

macro_rules! peel_impl_thread_builder {
    (($t:ident, $($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl_thread_builder!{ ($($ts,)*), ($($us,)*) }
    };
}

macro_rules! impl_thread_builder {
    ((), ()) => {};
    (($($ts:ident,)+), ($($channels:ident,)+)) => {
        impl<$($ts: Container + Send + 'static,)+> ThreadBuilder for ($($ts,)+) {
            type Threads = thread_builder_threads_type!((), ($($ts,)+), ());

            #[allow(non_snake_case)]
            fn start_threads(self) -> Vec<thread::Result<()>> {
                let channels = <Self::Threads as ChannelBuilder>::get_channels();
                let channels = channels.transpose();
                let channels = ChannelArranger::arrange(channels);
                let ($($ts,)+) = self;
                let ($($channels,)+) = channels;
                thread::scope(|s| {
                    let handles = vec![$(ThreadImpl {
                        container: $ts,
                        receiver: $channels.0,
                        senders: $channels.1,
                    }.spawn(s),)+];
                    handles.into_iter().map(|h| h.join()).collect()
                })
            }
        }
        peel_impl_thread_builder!{($($ts,)+), ($($channels,)+)}
    }
}

impl_thread_builder! {
    (A1, B1, C1, D1, E1, F1, G1, H1, I1, J1,),
    (A2, B2, C2, D2, E2, F2, G2, H2, I2, J2,)
}

trait ChannelBuilder {
    type Channels;
    fn get_channels() -> Self::Channels;
}

macro_rules! peel_channel_builder {
    () => {};
    ($t:ident, $($ts:ident,)*) => {
        impl_channel_builder!{$($ts,)*}
    }
}

macro_rules! impl_channel_builder {
    () => {};
    ($($ts:ident,)*) => {
        impl<$($ts:Thread + Send,)*> ChannelBuilder for ($($ts,)*) {
            type Channels = ($(Channel<$ts>,)*);
            fn get_channels() -> Self::Channels {
                ($(<$ts as Thread>::make_channel(),)*)
            }
        }
        peel_channel_builder!{$($ts,)*}
    }
}

impl_channel_builder! {A, B, C, D, E, F, G, H, I, J,}

trait ChannelArranger {
    type ArrangedChannels;
    fn arrange(self) -> Self::ArrangedChannels;
}

macro_rules! channel_arranger_output_helper {
    ((), ($($prev:ident,)*), (), ($($accum:ty,)*) ) => {
        ($($accum,)*)
    };
    (($recv:ident, $($recvs:ident,)*), ($($prev:ident,)*), ($t:ident, $($ts:ident,)*), ($($accum:ty,)*)) => {
        channel_arranger_output_helper!(($($recvs,)*), ($($prev,)* $t,), ($($ts,)*), ($($accum,)* ($recv, ($($prev,)* $($ts,)*)),))
    };
}

macro_rules! channel_arranger_impl_helper {
    ((), ($($prev:ident,)*), (), ($($accum:tt,)*)) => {
        ($($accum,)*)
    };
    (($receiver:ident, $($receivers:ident,)*), ($($prev:ident,)*), ($t:ident, $($ts:ident,)*), ($($accum:tt,)*)) => {
        channel_arranger_impl_helper!(($($receivers,)*), ($($prev,)* $t,), ($($ts,)*),
            ($($accum,)*
            ({
                ($receiver, ($($prev.clone(),)* $($ts.clone(),)*))
            }),)
        )
    };
}

macro_rules! peel_impl_channel_arranger {
    ((), ()) => {};
    (($t:ident, $($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl_channel_arranger!{ ($($ts,)*), ($($us,)*) }
    };
}
macro_rules! impl_channel_arranger {
    (($($senders:ident,)*), ($($receivers:ident,)*)) => {
        impl<$($senders:Clone,)* $($receivers,)*> ChannelArranger for (($($senders,)*), ($($receivers,)*)) {
            type ArrangedChannels = channel_arranger_output_helper!(($($receivers,)*), (), ($($senders,)*), ());
            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn arrange(self) -> Self::ArrangedChannels {
                let (($($senders,)*), ($($receivers,)*)) = self;
                channel_arranger_impl_helper!(($($receivers,)*), (), ($($senders,)*), ())
            }
        }
        peel_impl_channel_arranger!{($($senders,)*), ($($receivers,)*)}
    }
}

impl_channel_arranger! {
    (A1, B1, C1, D1, E1, F1, G1, H1, I1, J1,),
    (A2, B2, C2, D2, E2, F2, G2, H2, I2, J2,)
}

trait Transpose2 {
    type Output;

    fn transpose(self) -> Self::Output;
}

macro_rules! peel_transpose2 {
    ((), ()) => {};
    (($t:ident, $($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl_transpose2!{ ($($ts,)*), ($($us,)*) }
    };
}

macro_rules! impl_transpose2 {
    (($($ts:ident,)*), ($($us:ident,)*)) => {
        #[allow(non_snake_case)]
        impl<$($ts,)* $($us,)*> Transpose2 for ($(($ts, $us,),)*) {
            type Output = (($($ts,)*), ($($us,)*),);

            fn transpose(self) -> Self::Output {
                let ($(($ts, $us,),)*) = self;
                (($($ts,)*), ($($us,)*))
            }
        }
        peel_transpose2!{ ($($ts,)*), ($($us,)*) }
    };
}

impl_transpose2! {
    (A1, B1, C1, D1, E1, F1, G1, H1, I1, J1,),
    (A2, B2, C2, D2, E2, F2, G2, H2, I2, J2,)
}
