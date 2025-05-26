use zerocopy::*;

#[repr(transparent)]
#[derive(IntoBytes, FromZeros, KnownLayout, Immutable, Unaligned)]
#[derive(Clone, Copy, Eq)]
pub struct FixedBString<const N: usize>([u8; N]);

impl<const N: usize> FixedBString<N> {
    pub const fn new() -> Self {
        Self([0; N])
    }

    pub fn from_truncated(mut s: &str) -> Self {
        if s.len() > N {
            s = &s[..N];
        }
        Self::try_from(s).expect("from_truncated should never result in error")
    }

    /// This function is O(N).
    fn to_slice(&self) -> &[u8] {
        let end = self.0.iter().position(|&b| b == 0).unwrap_or(N);
        &self.0[..end]
    }

    fn bytes(&self) -> impl Iterator<Item = u8> {
        self.0.into_iter().take_while(|&b| b != 0)
    }
}

impl<const N: usize> Default for FixedBString<N> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LengthError;

impl<const N: usize> std::convert::TryFrom<&str> for FixedBString<N> {
    type Error = LengthError;

    fn try_from(inp: &str) -> Result<Self, Self::Error> {
        let mut outp = Self::new();
        for (i, c) in inp.bytes().enumerate() {
            outp.0[i] = c;
        }
        Ok(outp)
    }
}

impl<const N: usize> std::hash::Hash for FixedBString<N> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write(self.to_slice())
    }
}

impl<const N: usize> std::fmt::Display for FixedBString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.bytes() {
            write!(f, "{}", b as char)?;
        }
        Ok(())
    }
}

impl<const N: usize> std::fmt::Debug for FixedBString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"")?;
        for b in self.0 {
            write!(f, "{}", b as char)?;
        }
        write!(f, "\"")?;
        Ok(())
    }
}

impl<T, const N: usize> core::cmp::PartialEq<T> for FixedBString<N>
where T: AsRef<[u8]> + ?Sized
{
    fn eq(&self, other: &T) -> bool {
        self.bytes().eq(other.as_ref().iter().copied().take_while(|&b| b != 0))
    }
}

impl<const N: usize, const M: usize> core::cmp::PartialEq<FixedBString<N>> for FixedBString<M> {
    fn eq(&self, other: &FixedBString<N>) -> bool {
        self.bytes().eq(other.bytes())
    }
}

impl<const N: usize, const M: usize> core::cmp::PartialOrd<FixedBString<N>> for FixedBString<M> {
    fn partial_cmp(&self, other: &FixedBString<N>) -> Option<std::cmp::Ordering> {
        self.bytes().partial_cmp(other.bytes())
    }
}

impl<T, const N: usize> core::cmp::PartialOrd<T> for FixedBString<N>
where T: AsRef<[u8]> + ?Sized
{
    fn partial_cmp(&self, other: &T) -> Option<std::cmp::Ordering> {
        self.bytes().partial_cmp(other.as_ref().iter().copied().take_while(|&b| b != 0))
    }
}

impl<const N: usize> core::cmp::Ord for FixedBString<N> {
    fn cmp(&self, other: &FixedBString<N>) -> std::cmp::Ordering {
        self.bytes().cmp(other.bytes())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    type FS = FixedBString<40>;

    #[test]
    fn test() {
        let hello = FixedBString::<40>::try_from("Hello world").unwrap();
        assert_eq!(hello, "Hello world");
        assert_eq!(FixedBString(*b"Hello\0world"), FixedBString(*b"Hello"));
    }
    
    #[test]
    fn test_truncated() {
        assert_eq!(FixedBString::<5>::from_truncated("Hel"), "Hel");
        assert_eq!(FixedBString::<5>::from_truncated("Hello"), "Hello");
        assert_eq!(FixedBString::<5>::from_truncated("Hel\0o"), "Hel");
        assert_eq!(FixedBString::<5>::from_truncated("Hello world"), "Hello");
    }

    #[test]
    fn test_fmt() {
        assert_eq!(format!("{}", FS::try_from("Hello world").unwrap()), "Hello world");
    }

    #[test]
    fn test_cmp() {
        assert_eq!(FS::from_truncated("Hello world"), FS::from_truncated("Hello world\0xyz"));
        assert_eq!(FS::try_from("Hello world").unwrap(), "Hello world\0xyz");
        assert_ne!(FS::try_from("Hello world").unwrap(), FS::from_truncated("Hello world!"));
        assert_ne!(FS::try_from("Hello world").unwrap(), "Hello world!");
        assert!(FS::try_from("Hello worlc").unwrap() < "Hello world\0xyz");
        assert!(FS::try_from("Hello worl\0z").unwrap() < "Hello world\0xyz");
    }

    #[test]
    #[should_panic]
    fn test_len_err() {
        let _hello = FixedBString::<10>::try_from("Hello world").unwrap();
    }
}
