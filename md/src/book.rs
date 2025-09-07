use crate::Side;
use core::ops::AddAssign;
use std::collections::BTreeMap;

pub trait BookHandler {
    type UserData;
}

pub trait BookTraits {
    type OrderId;
    type Quantity: Default + Eq + Copy + AddAssign;
    type Price: Ord + Eq + Copy + Default;
}

pub struct OrderBook<Traits: BookTraits> {
    bids: BTreeMap<Traits::Price, Level<Traits>>,
    asks: BTreeMap<std::cmp::Reverse<Traits::Price>, Level<Traits>>,
}

struct Level<Traits: BookTraits> {
    seqno: u64,
    quantity: Traits::Quantity,
    price: Traits::Price,
}

impl<Traits> OrderBook<Traits>
where
    Traits: BookTraits,
{
    pub fn added(&mut self, seqno: u64, side: Side, quantity: Traits::Quantity, price: Traits::Price) -> bool {
        fn helper<T: Ord + Eq + Copy, Traits: BookTraits>(
            levels: &mut BTreeMap<T, Level<Traits>>,
            key: T,
            seqno: u64,
            quantity: Traits::Quantity,
            price: Traits::Price,
        ) -> bool {
            let level = levels.entry(key).or_insert_with(|| Level {
                seqno,
                quantity: Default::default(),
                price,
            });
            level.quantity += quantity;
            key == *levels.first_key_value().unwrap().0
        }
        match side {
            Side::Bid => helper(&mut self.bids, price, seqno, quantity, price),
            Side::Ask => helper(&mut self.asks, std::cmp::Reverse(price), seqno, quantity, price),
        }
    }
}
