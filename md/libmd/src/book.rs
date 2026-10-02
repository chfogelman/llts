#![allow(clippy::missing_assert_message)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, btree_map, hash_map};
use std::fmt::Debug;
use std::hash::Hash;

use crate::{L1Handler, MdTraits, Side, events};

pub struct Book<Traits>
where
    Traits: MdTraits,
{
    bids: BTreeMap<std::cmp::Reverse<Traits::Price>, Level<Traits>>,
    asks: BTreeMap<Traits::Price, Level<Traits>>,
}

impl<Traits: MdTraits> std::ops::Drop for Book<Traits> {
    fn drop(&mut self) {
        for level in self.bids.values() {
            assert!(level.quantity > Default::default());
        }
        for level in self.asks.values() {
            assert!(level.quantity > Default::default());
        }
    }
}

impl<Traits> Book<Traits>
where
    Traits: MdTraits,
{
    pub fn bids(&self) -> impl Iterator<Item = (Traits::Price, Traits::Quantity)> {
        self.bids.iter().map(|(px, lvl)| (px.0, lvl.quantity))
    }

    pub fn asks(&self) -> impl Iterator<Item = (Traits::Price, Traits::Quantity)> {
        self.asks.iter().map(|(&px, lvl)| (px, lvl.quantity))
    }

    #[must_use]
    pub fn get_top(&self) -> Option<events::Top<Traits>> {
        let bid = self.bids.first_key_value()?;
        let ask = self.asks.first_key_value()?;
        Some(events::Top {
            bid_size:  bid.1.quantity,
            bid_price: bid.0.0,
            ask_size:  ask.1.quantity,
            ask_price: *ask.0,
        })
    }
}

struct Level<Traits>
where
    Traits: MdTraits,
{
    seqno:    u64,
    quantity: Traits::Quantity,
}

struct BookWithData<Traits, UserData>
where
    Traits: MdTraits,
{
    book:      Book<Traits>,
    user_data: UserData,
}

impl<Traits, UserData> BookWithData<Traits, UserData>
where
    Traits: MdTraits,
{
    fn new(data: UserData) -> Self {
        Self {
            book:      Book {
                bids: BTreeMap::new(),
                asks: BTreeMap::new(),
            },
            user_data: data,
        }
    }

    #[must_use]
    fn add(&mut self, seqno: u64, side: Side, quantity: Traits::Quantity, price: Traits::Price) -> bool {
        fn helper<T: Ord + Eq + Copy, Traits: MdTraits>(
            levels: &mut BTreeMap<T, Level<Traits>>,
            key: T,
            seqno: u64,
            quantity: Traits::Quantity,
        ) -> bool {
            let level = levels.entry(key).or_insert_with(|| Level {
                seqno,
                quantity: Default::default(),
            });
            level.quantity += quantity;
            key == *levels.first_key_value().expect("An inhabited level").0
        }
        match side {
            Side::Bid => helper(&mut self.book.bids, Reverse(price), seqno, quantity),
            Side::Ask => helper(&mut self.book.asks, price, seqno, quantity),
        }
    }

    fn reduce(&mut self, seqno: u64, side: Side, quantity: Traits::Quantity, price: Traits::Price) -> bool {
        fn helper<T: Ord + Eq + Copy, Traits: MdTraits>(
            levels: &mut BTreeMap<T, Level<Traits>>,
            key: T,
            seqno: u64,
            quantity: Traits::Quantity,
        ) -> bool {
            let Some((&top_key, _)) = levels.first_key_value() else {
                return false;
            };
            if let btree_map::Entry::Occupied(mut level) = levels.entry(key) {
                level.get_mut().quantity -= quantity;
                level.get_mut().seqno = seqno;
                if level.get().quantity <= Default::default() {
                    assert_eq!(level.get().quantity, Default::default());
                    level.remove();
                }
            } else {
                panic!("Could not find level");
            }
            key == top_key
        }
        match side {
            Side::Bid => helper(&mut self.book.bids, Reverse(price), seqno, quantity),
            Side::Ask => helper(&mut self.book.asks, price, seqno, quantity),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct Order<Traits>
where
    Traits: MdTraits,
{
    price:    Traits::Price,
    quantity: Traits::Quantity,
    side:     Side,
    book_idx: u16,
    seqno:    u64,
}

pub struct Feed<Traits, Handler>
where
    Traits: MdTraits,
    Handler: L1Handler<Traits>,
{
    handler:      Handler,
    books:        Vec<BookWithData<Traits, Handler::UserData>>,
    inst_to_book: HashMap<Traits::InstrumentId, u16>,
    orders:       HashMap<Traits::OrderId, Order<Traits>>,
}

impl<Traits, Handler> std::ops::Drop for Feed<Traits, Handler>
where
    Traits: MdTraits,
    Handler: L1Handler<Traits>,
{
    fn drop(&mut self) {
        for order in self.orders.values() {
            assert!(order.quantity > Default::default());
        }
    }
}

impl<Traits, Handler> Feed<Traits, Handler>
where
    Traits: MdTraits,
    Handler: L1Handler<Traits>,
{
    pub fn new(handler: Handler) -> Self {
        Self {
            handler,
            books: Default::default(),
            inst_to_book: Default::default(),
            orders: Default::default(),
        }
    }

    pub fn subscribe(&mut self, instrument: Traits::InstrumentId, user_data: Handler::UserData) {
        self.inst_to_book.entry(instrument).or_insert_with(|| {
            self.books.push(BookWithData::new(user_data));
            (self.books.len() - 1) as u16
        });
    }

    pub fn added(
        &mut self,
        instrument: Traits::InstrumentId,
        seqno: u64,
        id: Traits::OrderId,
        side: Side,
        quantity: Traits::Quantity,
        price: Traits::Price,
    ) {
        let Some(&idx) = self.inst_to_book.get(&instrument) else {
            return;
        };
        let book = &mut self.books[idx as usize];
        if let Some(prev) = self.orders.insert(
            id,
            Order {
                price,
                quantity,
                side,
                book_idx: idx,
                seqno,
            },
        ) {
            panic!("Adding order ID {id} that replaces previous order {prev:?}");
        }
        if book.add(seqno, side, quantity, price) {
            self.handler.on_quote(&book.book, &mut book.user_data);
        }
    }

    pub fn executed(&mut self, seqno: u64, id: Traits::OrderId, executed_quantity: Traits::Quantity) {
        let hash_map::Entry::Occupied(mut order) = self.orders.entry(id) else {
            return;
        };
        let Order {
            price,
            quantity,
            side,
            book_idx,
            ..
        } = order.get_mut();
        let book = &mut self.books[*book_idx as usize];
        if book.reduce(seqno, *side, executed_quantity, *price) {
            self.handler.on_trade(&book.book, &mut book.user_data);
        }
        *quantity -= executed_quantity;
        if *quantity <= Default::default() {
            assert_eq!(*quantity, Default::default());
            order.remove();
        }
    }

    pub fn executed_at_price(
        &mut self,
        seqno: u64,
        id: Traits::OrderId,
        executed_quantity: Traits::Quantity,
        _executed_price: Traits::Price,
    ) {
        let hash_map::Entry::Occupied(mut order) = self.orders.entry(id) else {
            return;
        };
        let Order {
            price,
            quantity,
            side,
            book_idx,
            ..
        } = order.get_mut();
        let book = &mut self.books[*book_idx as usize];
        book.reduce(seqno, *side, executed_quantity, *price);
        self.handler.on_trade(&book.book, &mut book.user_data);
        *quantity -= executed_quantity;
        if *quantity <= Default::default() {
            assert_eq!(*quantity, Default::default());
            order.remove();
        }
    }

    pub fn replaced(
        &mut self,
        seqno: u64,
        old_id: Traits::OrderId,
        new_id: Traits::OrderId,
        new_quantity: Traits::Quantity,
        new_price: Traits::Price,
    ) {
        let Some(order) = self.orders.remove(&old_id) else {
            return;
        };
        let Order {
            price: old_price,
            quantity: old_quantity,
            side,
            book_idx,
            ..
        } = order;
        let book = &mut self.books[book_idx as usize];
        book.reduce(seqno, side, old_quantity, old_price);
        if book.add(seqno, side, new_quantity, new_price) {
            self.handler.on_quote(&book.book, &mut book.user_data);
        }
        self.orders.insert(
            new_id,
            Order {
                price: new_price,
                quantity: new_quantity,
                side,
                book_idx,
                seqno,
            },
        );
    }

    pub fn reduced(&mut self, seqno: u64, id: Traits::OrderId, canceled_quantity: Traits::Quantity) {
        let hash_map::Entry::Occupied(mut order) = self.orders.entry(id) else {
            return;
        };
        let Order {
            price,
            quantity,
            side,
            book_idx,
            ..
        } = order.get_mut();
        let book = &mut self.books[*book_idx as usize];
        book.reduce(seqno, *side, canceled_quantity, *price);
        self.handler.on_quote(&book.book, &mut book.user_data);
        *quantity -= canceled_quantity;
        if *quantity <= Default::default() {
            assert_eq!(*quantity, Default::default());
            order.remove();
        }
    }

    pub fn deleted(&mut self, id: Traits::OrderId) {
        let hash_map::Entry::Occupied(order) = self.orders.entry(id) else {
            return;
        };
        let Order {
            price,
            quantity,
            side,
            book_idx: book,
            seqno,
        } = *order.get();
        let book = &mut self.books[book as usize];
        book.reduce(seqno, side, quantity, price);
        self.handler.on_quote(&book.book, &mut book.user_data);
        order.remove();
    }
}
