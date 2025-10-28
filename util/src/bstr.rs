pub mod fixed_bstring;

pub use fixed_bstring::FixedString;

#[repr(transparent)]
pub struct BStr([u8]);

impl core::convert::From<&[u8]> for &BStr {
    fn from(value: &[u8]) -> Self {
        // SAFETY: cf. repr(transparent)
        unsafe { core::mem::transmute(value) }
    }
}
