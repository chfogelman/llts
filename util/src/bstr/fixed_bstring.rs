//! A fixed-length string type.
//!
//! This type serves as a fundamental stack-allocated string type.

use std::fmt::Write;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[repr(transparent)]
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Clone, Copy, Eq)]
pub struct FixedString<const N: usize, const E: u8 = 0>([u8; N]);

impl<const N: usize, const E: u8> FixedString<N, E> {
    #[must_use]
    pub const fn new() -> Self {
        Self([E; N])
    }

    #[must_use]
    pub fn from_truncated(mut s: &str) -> Self {
        if s.len() > N {
            s = &s[..N];
        }
        Self::try_from(s).expect("This string should have already been truncated!")
    }

    /// Returns the bytes of this string as a slice.
    ///
    /// Because it must find the terminating character, it is O(N) and gets a to_ naming convention
    fn to_slice(&self) -> &[u8] {
        #![allow(clippy::wrong_self_convention)]
        let end = self.0.iter().position(|&b| b == E).unwrap_or(N);
        &self.0[..end]
    }

    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.0.into_iter().take_while(|&b| b != E)
    }
}

impl<const N: usize> Default for FixedString<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LengthError;

/// Allows conversion from byte array literals, e.g. `b"Hello world".into()`
impl<const N: usize, const E: u8, const M: usize> std::convert::From<&[u8; M]> for FixedString<N, E> {
    fn from(value: &[u8; M]) -> Self {
        crate::const_assert_generic!(M, <=, N, "FixedString<N> cannot be created from larger bytestring");
        let mut out = Self::new();
        for (i, c) in value.iter().take_while(|&&b| b != E).enumerate() {
            out.0[i] = *c;
        }
        out
    }
}

impl<const N: usize, const E: u8> std::convert::TryFrom<&str> for FixedString<N, E> {
    type Error = LengthError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut out = Self::new();
        for (i, c) in value.bytes().take_while(|&b| b != E).enumerate() {
            if i < N {
                out.0[i] = c;
            } else {
                return Err(LengthError);
            }
        }
        Ok(out)
    }
}

impl<const N: usize, const E: u8> std::hash::Hash for FixedString<N, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // NOTE: this must remain the precise implementation because it matches that of [u8].
        // If not, the implementation of `Borrow` below is incorrect.
        let slc = self.to_slice();
        state.write_usize(slc.len());
        state.write(slc);
    }
}

impl<const N: usize, const E: u8> std::borrow::Borrow<[u8]> for FixedString<N, E> {
    fn borrow(&self) -> &[u8] {
        self.to_slice()
    }
}

impl<const N: usize, const E: u8> std::fmt::Display for FixedString<N, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.bytes() {
            f.write_char(b as char)?;
        }
        Ok(())
    }
}

impl<const N: usize, const E: u8> std::fmt::Debug for FixedString<N, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for b in self.0 {
            f.write_char(b as char)?;
        }
        write!(f, "\"fs<{N}>")
    }
}

impl<T, const N: usize, const E: u8> core::cmp::PartialEq<T> for FixedString<N, E>
where
    T: AsRef<[u8]> + ?Sized,
{
    fn eq(&self, other: &T) -> bool {
        self.bytes().eq(other.as_ref().iter().copied().take_while(|&b| b != E))
    }
}

impl<const N: usize, const M: usize, const E: u8> core::cmp::PartialEq<FixedString<N, E>> for FixedString<M, E> {
    fn eq(&self, other: &FixedString<N, E>) -> bool {
        self.bytes().eq(other.bytes())
    }
}

impl<const N: usize, const M: usize, const E: u8> core::cmp::PartialOrd<FixedString<N, E>> for FixedString<M, E> {
    fn partial_cmp(&self, other: &FixedString<N, E>) -> Option<std::cmp::Ordering> {
        self.bytes().partial_cmp(other.bytes())
    }
}

impl<T, const N: usize, const E: u8> core::cmp::PartialOrd<T> for FixedString<N, E>
where
    T: AsRef<[u8]> + ?Sized,
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.bytes()
            .partial_cmp(other.as_ref().iter().copied().take_while(|&b| b != E))
    }
}

impl<const N: usize, const E: u8> core::cmp::Ord for FixedString<N, E> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bytes().cmp(other.bytes())
    }
}

#[cfg(test)]
mod test;
