
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Decimal<const MANTISSA: usize = 0>(u32);

impl<const M: usize> Decimal<M> {

    const MULTIPLICAND: u32 = Self::scale();

    const fn scale() -> u32 {
        let mut i = 0;
        let mut res = 1;
        while i < M {
            res *= 10;
            i += 1;
        }
        res
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn split(self) -> (u32, u32) {
        (self.0 / Self::MULTIPLICAND, self.0 % Self::MULTIPLICAND)
    }
}

impl<const M: usize> From<u32> for Decimal<M> {
    fn from(value: u32) -> Self {
        Self(value * Self::MULTIPLICAND)
    }
}

impl<const M: usize> From<f64> for Decimal<M> {
    fn from(value: f64) -> Self {
        Self((value * Self::MULTIPLICAND as f64) as u32)
    }
}

impl<const M: usize> std::fmt::Display for Decimal<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (characteristic, mantissa) = self.split();
        write!(f, "{}.{:0width$}", characteristic, mantissa, width = M)
    }
}

impl<const M: usize> std::fmt::Debug for Decimal<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (characteristic, mantissa) = self.split();
        write!(f, "{:?}.{:?}", characteristic, mantissa)
    }
}

impl<const M: usize> std::ops::Add for Decimal<M> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<const M: usize> std::ops::Sub for Decimal<M> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<const M: usize> std::ops::Mul for Decimal<M> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0 / Self::MULTIPLICAND)
    }
}

impl<const M: usize> std::ops::Div for Decimal<M> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 * Self::MULTIPLICAND / rhs.0)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_construct() {
        let dec = Decimal::<3>::from(10.035);
        assert_eq!(dec.split(), (10, 35));

        let dec = Decimal::<5>::from(10);
        assert_eq!(dec.split(), (10, 0));
    }

    #[test]
    fn test_fmt() {
        assert_eq!(
            format!("{}", Decimal::<9>::from(1.000490048)),
            "1.000490048"
            );
    }

    #[test]
    fn test_add() {
        let dec1 = Decimal::<2>::from(10.45);
        let dec2 = Decimal::from(5.22);
        assert_eq!((dec1 + dec2).split(), (15, 67));
        
        let (dec1, dec2) = (Decimal::<2>::from(13.99), Decimal::from(0.02));
        assert_eq!((dec1 + dec2).split(), (14, 1));
    }

    #[test]
    fn test_sub() {
        let dec1 = Decimal::<2>::from(10.45);
        let dec2 = Decimal::from(5.22);
        assert_eq!((dec1 - dec2).split(), (5, 23));
        
        let (dec1, dec2) = (Decimal::<3>::from(14.010), Decimal::from(0.020));
        assert_eq!((dec1 - dec2).split(), (13, 990));
    }
}
