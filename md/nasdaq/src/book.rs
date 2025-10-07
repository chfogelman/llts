use std::collections::HashMap;

use libmd::L1Handler;
use libmd::book::Feed;

use crate::messages::*;
use crate::{NasdaqTraits, Price, Symbol};

pub struct BookBuilder<H: L1Handler<NasdaqTraits>> {
    books:         Feed<NasdaqTraits, H>,
    subscriptions: HashMap<Symbol, H::UserData>,
}

impl<H: L1Handler<NasdaqTraits>> BookBuilder<H> {
    pub fn new(handler: H) -> Self {
        Self {
            books:         Feed::new(handler),
            subscriptions: HashMap::new(),
        }
    }

    pub fn subscribe(&mut self, symbol: Symbol, user_data: H::UserData) {
        self.subscriptions.insert(symbol, user_data);
    }
}

impl<H: L1Handler<NasdaqTraits>> Handler for BookBuilder<H> {
    fn on_stock_directory(&mut self, directory: &StockDirectory) {
        println!("{:?}", directory);
        if let Some(entry) = self.subscriptions.remove_entry(&directory.stock) {
            self.books.subscribe(directory.header.stock_locate.get(), entry.1);
        }
    }

    fn on_stock_trading_action(&mut self, _action: &StockTradingAction) {}

    fn on_add_order(&mut self, add: &AddOrder) {
        self.books.added(
            add.header.stock_locate.get(),
            0,
            add.order_ref.get(),
            to_side(add.side),
            add.shares.get(),
            Price::from_raw(add.price.get() as i32),
        );
    }

    fn on_add_order_with_mpid(&mut self, add: &AddOrderWithMpid) {
        self.books.added(
            add.header.stock_locate.get(),
            0,
            add.order_ref.get(),
            to_side(add.side),
            add.shares.get(),
            Price::from_raw(add.price.get() as i32),
        );
    }

    fn on_order_executed(&mut self, executed: &OrderExecuted) {
        self.books
            .executed(0, executed.order_ref.get(), executed.executed_shares.get());
    }

    fn on_order_executed_with_price(&mut self, executed: &OrderExecutedWithPrice) {
        self.books.executed_at_price(
            0,
            executed.order_ref.get(),
            executed.executed_shares.get(),
            Price::from_raw(executed.execution_price.get() as i32),
        );
    }

    fn on_order_cancel(&mut self, cancel: &OrderCancel) {
        self.books
            .reduced(0, cancel.order_ref.get(), cancel.canceled_shares.get());
    }

    fn on_order_delete(&mut self, delete: &OrderDelete) {
        self.books.deleted(delete.order_ref.get());
    }

    fn on_order_replace(&mut self, replace: &OrderReplace) {
        self.books.replaced(
            0,
            replace.orig_order_ref.get(),
            replace.new_order_ref.get(),
            replace.shares.get(),
            Price::from_raw(replace.price.get() as i32),
        );
    }
}
