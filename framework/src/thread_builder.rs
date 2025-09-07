use crate::envelope::{Envelope, EnvelopeBuilder};
use crate::{Context, Handler, PollEvent, ThreadLocalEnvelope};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::thread;

pub struct Thread<Hlr: Send + 'static, Others: Hub + 'static> {
    component: Hlr,
    receiver: Receiver<Envelope<Self>>,
    senders: <Others as Hub>::Handles<Hlr>,
}

impl<Hlr: Send, H: Hub> Thread<Hlr, H> {
    fn poll(&mut self) {
        self.dispatch_msg(&PollEvent);
        let next = self.receiver.try_recv();
        match next {
            Ok(msg) => msg.dispatch_onto(self),
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => {
                eprintln!("Exiting due to disconnection");
                std::process::exit(-1);
            },
        }
    }
}

pub trait ThreadBuilder {
    type Threads;
    fn build_threads(self) -> Self::Threads;
}

pub trait Hub {
    type Handles<T: Send + 'static>: Context + Send + 'static;
}

type Channel<T> = (<T as ThreadInfo>::Sender, <T as ThreadInfo>::Receiver);

pub trait ThreadInfo {
    type Channel;
    type Sender;
    type Receiver;
    fn make_channel() -> Channel<Self>;
    fn dispatch_msg<Msg>(&mut self, msg: &Msg);
    fn spawn(self) -> thread::JoinHandle<()>;
}

impl<Hlr, B: Hub> ThreadInfo for Thread<Hlr, B>
where
    Hlr: Send,
{
    type Sender = Sender<Envelope<Self>>;
    type Receiver = Receiver<Envelope<Self>>;
    type Channel = Channel<Self>;

    fn make_channel() -> Channel<Self> {
        channel()
    }

    fn dispatch_msg<Msg>(&mut self, msg: &Msg) {
        <Hlr as Handler<Msg>>::handle(&mut self.component, &mut self.senders, msg);
    }

    fn spawn(self) -> thread::JoinHandle<()> {
        let mut thread_impl = self;
        thread::spawn(move || {
            loop {
                thread_impl.poll();
            }
        })
    }
}

type ThreadHandle<T> = Sender<Envelope<T>>;

macro_rules! peel_impl_context_for_handles {
    ($t:ident, $($ts:ident,)*) => {
        impl_context_for_handles!{$($ts,)*}
    }
}

macro_rules! impl_context_for_handles {
    () => {};
    ($($ts:ident,)+) => {
        impl<$($ts: ThreadInfo,)+> Context for ($(ThreadHandle<$ts>,)+) {
            #[allow(non_snake_case)]
            fn publish<Msg>(&mut self, msg: impl ThreadLocalEnvelope<Msg>) {
                let msg = msg.alloc_envelope();
                let ($($ts,)+) = self;
                $({$ts.send(msg.build_envelope()).unwrap()})+
            }
        }
        peel_impl_context_for_handles!{$($ts,)+}
    };
}

impl_context_for_handles! {A, B, C, D, E, F, G, H, I, J,}

macro_rules! impl_hub_type {
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), (), ($($accum:ty,)*)) => { ($($accum,)*) };
    ($slf:ident, ($($prev_xs:ident,)*), ($x:ident, $($xs:ident,)*), (), ($($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_hub_type!($slf, ($($prev_xs,)* $x,), ($($xs,)*), (), ($($ys,)*),
            ($($accum,)* ThreadHandle<Thread<$x, (($($prev_xs,)*), ($($xs,)* $slf, $($ys,)*))>>,))
    };
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), ($y:ident, $($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_hub_type!($slf, ($($prev_xs,)*), (), ($($prev_ys,)* $y,), ($($ys,)*),
            ($($accum,)* ThreadHandle<Thread<$y, (($($prev_xs,)* $slf, $($prev_ys,)*), ($($ys,)*))>>,))
    };
}

macro_rules! impl_hubs {
    (($($ts:ident,)*), ()) => {};
    (($($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl<$($ts: Send + 'static,)* $($us: Send + 'static,)*> Hub for (($($ts,)*), ($($us,)*)) {
            type Handles<$u: Send + 'static> = impl_hub_type!{$u, (), ($($ts,)*), (), ($($us,)*), ()};
        }
        impl_hubs!{($($ts,)* $u,), ($($us,)*)}
    };
}

macro_rules! impl_all_hubs {
    () => {};
    ($t:ident, $($ts:ident,)*) => {
        impl_hubs!{(), ($t, $($ts,)*)}
        impl_all_hubs!{$($ts,)*}
    };
}

impl_all_hubs! {A, B, C, D, E, F, G, H, I, J,}

macro_rules! thread_builder_threads_type {
    ( ($($prev:ident,)*), (), ($($accum:ty,)*) ) => {
        ($($accum,)*)
    };
    ( ($($prev:ident,)*), ($t:ident, $($ts:ident,)*), ($($accum:ty,)*) ) => {
        thread_builder_threads_type!(($($prev,)* $t,), ($($ts,)*), ($($accum,)* Thread<$t, (($($prev,)*), ($($ts,)*))>,))
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
        impl<$($ts: Send + 'static,)+> ThreadBuilder for ($($ts,)+) {
            type Threads = thread_builder_threads_type!((), ($($ts,)+), ());

            #[allow(non_snake_case)]
            fn build_threads(self) -> Self::Threads {
                let channels = <Self::Threads as ChannelBuilder>::get_channels();
                let channels = channels.transpose();
                let channels = ChannelArranger::arrange(channels);
                let ($($ts,)+) = self;
                let ($($channels,)+) = channels;
                ($(Thread{
                    component: $ts,
                    receiver: $channels.0,
                    senders: $channels.1,
                },)+)
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
        impl<$($ts:ThreadInfo + Send,)*> ChannelBuilder for ($($ts,)*) {
            type Channels = ($(Channel<$ts>,)*);
            fn get_channels() -> Self::Channels {
                ($(<$ts as ThreadInfo>::make_channel(),)*)
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

pub trait ThreadStart {
    fn start(self) -> Vec<thread::JoinHandle<()>>;
}

macro_rules! peel_impl_thread_start {
    ($t:ident, $($ts:ident,)*) => { impl_thread_start!{$($ts,)*} };
}

macro_rules! impl_thread_start {
    () => {};
    ($($ts:ident,)+) => {
        #[allow(non_snake_case)]
        impl<$($ts: ThreadInfo,)+> ThreadStart for ($($ts,)+) {
            fn start(self) -> Vec<thread::JoinHandle<()>> {
                let ($($ts,)+) = self;
                vec![$($ts.spawn(),)+]
            }
        }
        peel_impl_thread_start!{$($ts,)+}
    }
}

impl_thread_start! {A, B, C, D, E, F, G, H, I, J,}
