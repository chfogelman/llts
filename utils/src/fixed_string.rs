
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Debug)]
pub struct FixedString<const N: usize>([u8; N]);

impl<const N: usize> FixedString<N> {
    pub fn new(s: &str) -> Self {
        assert!(s.len() <= N);
        let mut init = Self([0; N]);
        for (i, c) in s.bytes().enumerate() {
            init.0[i] = c;
        }
        init
    }

    // Returns the length of the contained string
    // This is optimized for strings that take up at least half the size of the backing buffer
    pub fn len(&self) -> usize {
        self.0.iter()
            .rposition(|&b| b != 0)
            .map(|i| i + 1)
            .unwrap_or(0)
    }

    /// Returns true if the string is empty.
    pub fn is_empty(&self) -> bool {
        self.0[0] == 0
    }

    /// Returns a view into the contained string.
    /// This is somewhat expensive, as we must find the last non-NUL byte in the string.
    pub fn to_str(&self) -> &str {
        // SAFETY:
        // - The length is never greater than the length of the underlying buffer
        // - The contents are always valid utf-8 by construction.
        unsafe {
            let slc = std::slice::from_raw_parts(self.0.as_ptr(), self.len());
            std::str::from_utf8_unchecked(slc)
        }
    }
}

impl<const N: usize> std::fmt::Display for FixedString<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test() {
        let hello = FixedString::<40>::new("Hello world");
        assert_eq!(hello.to_str().len(), 11);
        assert_eq!(hello.to_str(), "Hello world");
    }

    #[test]
    #[should_panic]
    fn test_len_assert() {
        let _hello = FixedString::<10>::new("Hello world");
    }
}
