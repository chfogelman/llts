use crate::MdTraits;

pub struct Top<Traits>
where
    Traits: MdTraits,
{
    pub bid_size:  Traits::Quantity,
    pub bid_price: Traits::Price,
    pub ask_size:  Traits::Quantity,
    pub ask_price: Traits::Price,
}
