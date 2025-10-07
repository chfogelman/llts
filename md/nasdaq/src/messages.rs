use ::util::bstr::fixed_bstring::FixedString;
use libmd::Side;
use zerocopy::{network_endian as ne, *};

use crate::Symbol;

pub mod message_type {
    pub const STOCK_DIRECTORY: u8 = b'R';
    pub const STOCK_TRADING_ACTION: u8 = b'H';
    pub const ADD_ORDER: u8 = b'A';
    pub const ADD_ORDER_WITH_MPID: u8 = b'F';
    pub const ORDER_EXECUTED: u8 = b'E';
    pub const ORDER_EXECUTED_WITH_PRICE: u8 = b'C';
    pub const ORDER_CANCEL: u8 = b'X';
    pub const ORDER_DELETE: u8 = b'D';
    pub const ORDER_REPLACE: u8 = b'U';
}

#[allow(unused_variables)]
pub trait Handler {
    fn on_stock_directory(&mut self, directory: &StockDirectory);
    fn on_stock_trading_action(&mut self, action: &StockTradingAction);
    fn on_add_order(&mut self, add: &AddOrder);
    fn on_add_order_with_mpid(&mut self, add: &AddOrderWithMpid);
    fn on_order_executed(&mut self, executed: &OrderExecuted);
    fn on_order_executed_with_price(&mut self, executed: &OrderExecutedWithPrice);
    fn on_order_cancel(&mut self, cancel: &OrderCancel);
    fn on_order_delete(&mut self, delete: &OrderDelete);
    fn on_order_replace(&mut self, replace: &OrderReplace);
}

pub fn dispatch<H: Handler>(handler: &mut H, buf: &[u8]) {
    match buf[0] {
        message_type::STOCK_DIRECTORY => handler.on_stock_directory(StockDirectory::ref_from_bytes(buf).unwrap()),
        message_type::STOCK_TRADING_ACTION => {
            handler.on_stock_trading_action(StockTradingAction::ref_from_bytes(buf).unwrap())
        },
        message_type::ADD_ORDER => handler.on_add_order(AddOrder::ref_from_bytes(buf).unwrap()),
        message_type::ADD_ORDER_WITH_MPID => {
            handler.on_add_order_with_mpid(AddOrderWithMpid::ref_from_bytes(buf).unwrap())
        },
        message_type::ORDER_EXECUTED => handler.on_order_executed(OrderExecuted::ref_from_bytes(buf).unwrap()),
        message_type::ORDER_EXECUTED_WITH_PRICE => {
            handler.on_order_executed_with_price(OrderExecutedWithPrice::ref_from_bytes(buf).unwrap())
        },
        message_type::ORDER_CANCEL => handler.on_order_cancel(OrderCancel::ref_from_bytes(buf).unwrap()),
        message_type::ORDER_DELETE => handler.on_order_delete(OrderDelete::ref_from_bytes(buf).unwrap()),
        message_type::ORDER_REPLACE => handler.on_order_replace(OrderReplace::ref_from_bytes(buf).unwrap()),
        _ => {},
    }
}

pub fn to_side(buy_sell: u8) -> Side {
    match buy_sell {
        b'B' => Side::Bid,
        _ => Side::Ask,
    }
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct Header {
    pub message_type:    u8,
    pub stock_locate:    ne::U16,
    pub tracking_number: ne::U16,
    pub timestamp:       [u8; 6],
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct StockDirectory {
    pub header: Header,
    pub stock: Symbol,
    pub market_category: u8,
    pub status_indicator: u8,
    pub lot_size: ne::U32,
    pub round_lots_only: u8,
    pub issue_classification: u8,
    pub issue_subtype: ne::U16,
    pub authenticity: u8,
    pub short_sale_threshold_indicator: u8,
    pub ipo_flag: u8,
    pub ref_price_tier: u8,
    pub etp_flag: u8,
    pub etp_factor: ne::U32,
    pub inverse_indicator: u8,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct StockTradingAction {
    pub header:        Header,
    pub stock:         Symbol,
    pub trading_state: u8,
    _reserved:         u8,
    pub reason:        ne::U32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct AddOrder {
    pub header:    Header,
    pub order_ref: ne::U64,
    pub side:      u8,
    pub shares:    ne::U32,
    pub stock:     Symbol,
    pub price:     ne::U32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct AddOrderWithMpid {
    pub header:      Header,
    pub order_ref:   ne::U64,
    pub side:        u8,
    pub shares:      ne::U32,
    pub stock:       Symbol,
    pub price:       ne::U32,
    pub attribution: FixedString<4, b' '>,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct OrderExecuted {
    pub header:          Header,
    pub order_ref:       ne::U64,
    pub executed_shares: ne::U32,
    pub match_number:    ne::U64,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct OrderExecutedWithPrice {
    pub header:          Header,
    pub order_ref:       ne::U64,
    pub executed_shares: ne::U32,
    pub match_number:    ne::U64,
    pub printable:       u8,
    pub execution_price: ne::U32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct OrderCancel {
    pub header:          Header,
    pub order_ref:       ne::U64,
    pub canceled_shares: ne::U32,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct OrderDelete {
    pub header:    Header,
    pub order_ref: ne::U64,
}

#[repr(C)]
#[derive(FromBytes, IntoBytes, Immutable, Unaligned, KnownLayout, Clone, Copy, Debug)]
pub struct OrderReplace {
    pub header:         Header,
    pub orig_order_ref: ne::U64,
    pub new_order_ref:  ne::U64,
    pub shares:         ne::U32,
    pub price:          ne::U32,
}

const _: () = assert!(std::mem::size_of::<StockDirectory>() == 39);
const _: () = assert!(std::mem::size_of::<StockTradingAction>() == 25);
const _: () = assert!(std::mem::size_of::<AddOrder>() == 36);
const _: () = assert!(std::mem::size_of::<AddOrderWithMpid>() == 40);
const _: () = assert!(std::mem::size_of::<OrderExecuted>() == 31);
const _: () = assert!(std::mem::size_of::<OrderExecutedWithPrice>() == 36);
const _: () = assert!(std::mem::size_of::<OrderCancel>() == 23);
const _: () = assert!(std::mem::size_of::<OrderDelete>() == 19);
const _: () = assert!(std::mem::size_of::<OrderReplace>() == 35);
