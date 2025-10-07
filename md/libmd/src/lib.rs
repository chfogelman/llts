pub mod book;
pub mod events;
use core::ops::{AddAssign, SubAssign};
use std::fmt::{Debug, Display};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

pub trait MdTraits: Debug {
    type Symbol: Hash + Eq;
    type OrderId: Copy + Hash + Eq + Debug + Display;
    type InstrumentId: Copy + Hash + Eq + Debug + Display;
    type Quantity: Copy + Default + Eq + Copy + AddAssign + SubAssign + PartialOrd + Debug + Display;
    type Price: Copy + Ord + Eq + Copy + Default + Debug + Display;
}

pub trait L1Handler<Traits: MdTraits> {
    type UserData;

    fn on_quote(&mut self, book: &book::Book<Traits>, user_data: &mut Self::UserData);
    fn on_trade(&mut self, book: &book::Book<Traits>, user_data: &mut Self::UserData);
}
