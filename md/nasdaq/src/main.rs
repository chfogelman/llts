#![allow(clippy::unwrap_used)]

use libmd::book::Book;
use nasdaq::binfile;
use nasdaq::book::BookBuilder;

struct Printer;

impl<Traits: libmd::MdTraits> libmd::L1Handler<Traits> for Printer {
    type UserData = (String, i64, i64);

    fn on_quote(&mut self, book: &Book<Traits>, user_data: &mut Self::UserData) {
        user_data.1 += 1;
        let top = book.get_top();
        if let Some(top) = top {
            assert!(top.bid_price < top.ask_price, "Crossed or locked book");
            if user_data.1 % 10_000 == 0 {
                println!(
                    "Quote ${},{} | ${},{} for {}",
                    top.bid_price, top.bid_size, top.ask_price, top.ask_size, user_data.0
                );
            }
        }
    }

    fn on_trade(&mut self, _book: &Book<Traits>, user_data: &mut Self::UserData) {
        user_data.2 += 1;
    }
}

fn main() {
    let mut args = std::env::args();
    args.next().unwrap();
    let filename = args.next().expect("No file");
    println!("{}", filename);
    let file = std::fs::File::open(filename).expect("Cannot open file");
    let reader = flate2::read::GzDecoder::new(file);
    let mut handler = BookBuilder::new(Printer);
    handler.subscribe("AAPL".try_into().unwrap(), ("AAPL".into(), 0, 0));
    handler.subscribe("XLE".try_into().unwrap(), ("XLE".into(), 0, 0));
    handler.subscribe("TSLA".try_into().unwrap(), ("TSLA".into(), 0, 0));
    handler.subscribe("YELP".try_into().unwrap(), ("YELP".into(), 0, 0));
    let mut framer = binfile::BinFileFramer::new(reader, handler);

    let mut iters = 0;
    while framer.poll().unwrap().is_continue() {
        iters += 1;
    }
    println!("Num messages: {}", iters);
}
