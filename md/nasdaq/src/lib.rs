pub mod binfile;
pub mod book;
pub mod messages;

pub type Price = util::Fp32<4>;
pub type Symbol = util::FixedString<8, b' '>;

#[derive(Debug)]
pub struct NasdaqTraits;

impl libmd::MdTraits for NasdaqTraits {
    type Symbol = Symbol;
    type OrderId = u64;
    type InstrumentId = u16;
    type Quantity = u32;
    type Price = Price;
}
