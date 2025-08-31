#![allow(unused_imports)] // FIXME
use std::marker::PhantomData as Pd;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

struct ThreadImpl<A, Others: Hub> {
    a: Pd<A>,
    b: <Others as Hub>::Handles<Self>
}

trait Hub {
    type Handles<T: Thread>;
}

trait Thread {
    type Component;
    type Channel;
    type Sender;
    type Receiver;
    fn get_channel() -> Channel<Self>;
}

type Channel<T> = (<T as Thread>::Sender, <T as Thread>::Receiver);

impl<A, B: Hub> Thread for ThreadImpl<A, B> {
    type Component = A;

    type Sender = Sender<Arc<dyn Dispatcher<Self>>>;
    type Receiver = Receiver<Arc<dyn Dispatcher<Self>>>;

    type Channel = Channel<Self>;

    fn get_channel() -> Channel<Self> {
        channel()
    }
}


trait Dispatcher<T> {}

type ThreadHandle<T> = Sender<Arc<dyn Dispatcher<T>>>;

macro_rules! impl_hub_type {
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), (), ($($accum:ty,)*)) => { ($($accum,)*) };
    ($slf:ident, ($($prev_xs:ident,)*), ($x:ident, $($xs:ident,)*), (), ($($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_hub_type!($slf, ($($prev_xs,)* $x,), ($($xs,)*), (), ($($ys,)*),
            ($($accum,)* ThreadHandle<ThreadImpl<$x, (($($prev_xs,)*), ($($xs,)* <$slf as Thread>::Component, $($ys,)*))>>,))
    };
    ($slf:ident, ($($prev_xs:ident,)*), (), ($($prev_ys:ident,)*), ($y:ident, $($ys:ident,)*), ($($accum:ty,)*)) => {
        impl_hub_type!($slf, ($($prev_xs,)*), (), ($($prev_ys,)* $y,), ($($ys,)*),
            ($($accum,)* ThreadHandle<ThreadImpl<$y, (($($prev_xs,)* <$slf as Thread>::Component, $($prev_ys,)*), ($($ys,)*))>>,))
    };
}

macro_rules! impl_hubs {
    (($($ts:ident,)*), ()) => {};
    (($($ts:ident,)*), ($u:ident, $($us:ident,)*)) => {
        impl<$($ts,)* $($us,)*> Hub for (($($ts,)*), ($($us,)*)) {
            type Handles<$u: Thread> = impl_hub_type!{$u, (), ($($ts,)*), (), ($($us,)*), ()};
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

impl_all_hubs!{A, B, C, D, E, F, G, H, I, J,}

trait ThreadBuilder {
    type Threads;

    fn build_threads(self) -> Self::Threads;
}

macro_rules! thread_builder_threads_type {
    ( ($($prev:ident,)*), (), ($($accum:ty,)*) ) => {
        ($($accum,)*)
    };
    ( ($($prev:ident,)*), ($t:ident, $($ts:ident,)*), ($($accum:ty,)*) ) => {
        thread_builder_threads_type!(($($prev,)* $t,), ($($ts,)*), ($($accum,)* ThreadImpl<$t, (($($prev,)*), ($($ts,)*))>,))
    };
}

macro_rules! peel_impl_thread_builder {
    () => {};
    ($t:ident, $($ts:ident,)*) => {
        impl_thread_builder!{$($ts,)*}
    }
}


macro_rules! impl_thread_builder {
    () => {};
    ($($ts:ident,)+) => {
        impl<$($ts,)+> ThreadBuilder for ($($ts,)+) {
            type Threads = thread_builder_threads_type!((), ($($ts,)+), ());

            fn build_threads(self) -> Self::Threads {
                let channels = <Self::Threads as ChannelBuilder>::get_channels();
            }
        }
        peel_impl_thread_builder!{$($ts,)+}
    }
}

impl_thread_builder!{A, B, C, D, E, F, G, H, I, J,}

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
        impl<$($ts:Thread,)*> ChannelBuilder for ($($ts,)*) {
            type Channels = <($(Channel<$ts>,)*) as Transpose2>::Output;
            fn get_channels() -> Self::Channels {
                ($(<$ts as Thread>::get_channel(),)*).transpose()
            }
        }
        peel_channel_builder!{$($ts,)*}
    }
}

impl_channel_builder!{A, B, C, D, E, F, G, H, I, J,}

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
        peel_impl_channel_arranger!{ ($($ts,)*), ($($us,)*) }
    };
}
macro_rules! impl_channel_arranger {
    ((), ()) => {};
    (($($senders:ident,)*), ($($receivers:ident,)*)) => {
        impl<$($senders:Clone,)* $($receivers,)*> ChannelArranger for (($($senders,)*), ($($receivers,)*)) {
            type ArrangedChannels = channel_arranger_output_helper!(($($receivers,)*), (), ($($senders,)*), ());
            #[allow(non_snake_case)]
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
