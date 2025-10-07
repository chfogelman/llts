use zerocopy::*;

macro_rules! make_fixed_point { ($($name:ident, $int:ty)+) => {$(

#[repr(transparent)]
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct $name<const EXPONENT: $int = 0>(pub(self) $int);

impl<const E: $int> $name<E> {

    pub const MULTIPLICAND: $int = Self::scale();

    const fn scale() -> $int {
        let mut i = 0;
        let mut res = 1;
        while i < E {
            res *= 10;
            i += 1;
        }
        res
    }

    pub const fn zero() -> Self {
        Self(0)
    }

    pub const fn split(self) -> ($int, Self) {
        (self.0 / Self::MULTIPLICAND, Self(self.0 % Self::MULTIPLICAND))
    }

    pub const fn into_raw(self) -> $int {
        self.0
    }

    pub const fn from_raw(val: $int) -> Self {
        Self(val)
    }
}

impl<const E: $int> Default for $name<E> {
    fn default() -> Self {
        Self(0)
    }
}

impl<const M: $int> From<$int> for $name<M> {
    fn from(value: $int) -> Self {
        Self(value * Self::MULTIPLICAND)
    }
}

impl<const M: $int> From<f64> for $name<M> {
    fn from(value: f64) -> Self {
        Self((value * Self::MULTIPLICAND as f64) as $int)
    }
}

impl<const M: $int> std::fmt::Display for $name<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (characteristic, mantissa) = self.split();
        write!(f, "{:?}.{:0width$}", characteristic, mantissa.0.abs(), width = M as usize)
    }
}

impl<const M: $int> std::fmt::Debug for $name<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (characteristic, mantissa) = self.split();
        write!(f, "{}.{:0width$}:fp{}", characteristic, mantissa.0.abs(), M, width = M as usize)
    }
}

impl<const M: $int> std::ops::Add for $name<M> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<const M: $int> std::ops::AddAssign for $name<M> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}


impl<const M: $int> std::ops::Sub for $name<M> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<const M: $int> std::ops::SubAssign for $name<M> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs
    }
}


impl<const M: $int> std::ops::Mul for $name<M> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0 / Self::MULTIPLICAND)
    }
}

impl<const M: $int> std::ops::MulAssign for $name<M> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<const M: $int> std::ops::Div for $name<M> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self(self.0 * Self::MULTIPLICAND / rhs.0)
    }
}

)+}}

make_fixed_point!(Fp8, i8);
make_fixed_point!(Fp16, i16);
make_fixed_point!(Fp32, i32);
make_fixed_point!(Fp64, i64);
make_fixed_point!(Fp128, i128);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_construct() {
        let dec = Fp64::<3>::from(10.035);
        assert_eq!(dec.split(), (10, 0.035.into()));

        let dec = Fp64::<5>::from(10);
        assert_eq!(dec.split(), (10, 0.into()));
    }

    #[test]
    fn test_fmt() {
        assert_eq!(format!("{}", Fp64::<9>::from(-1.00049004)), "-1.000490040");
        assert_eq!(format!("{:?}", Fp64::<9>::from(-1.00049004)), "-1.000490040:fp9");
    }

    #[test]
    fn test_add() {
        let dec1 = Fp64::<2>::from(10.45);
        let dec2 = Fp64::from(5.22);
        assert_eq!((dec1 + dec2).split(), (15, 0.67.into()));

        let (dec1, dec2) = (Fp64::<2>::from(13.99), Fp64::from(0.02));
        assert_eq!((dec1 + dec2).split(), (14, 0.01.into()));
    }

    #[test]
    fn test_sub() {
        let dec1 = Fp64::<2>::from(10.45);
        let dec2 = Fp64::from(5.22);
        assert_eq!((dec1 - dec2).split(), (5, 0.23.into()));

        let (dec1, dec2) = (Fp64::<3>::from(14.010), Fp64::from(0.020));
        assert_eq!((dec1 - dec2).split(), (13, 0.990.into()));
    }

    #[test]
    fn test_mul() {
        let dec1 = Fp64::<2>::from(10.45);
        let dec2 = Fp64::from(5.22);
        assert_eq!(dec1 * dec2, 54.54.into());

        let (dec1, dec2) = (Fp64::<3>::from(14.010), Fp64::from(0.020));
        assert_eq!((dec1 * dec2).split(), (0, 0.280.into()));
    }

    #[test]
    fn test_div() {
        let dec1 = Fp64::<2>::from(9.45);
        let dec2 = Fp64::from(5.22);
        assert_eq!(dec1 / dec2, 1.80.into());

        let (dec1, dec2) = (Fp64::<3>::from(14.010), Fp64::from(0.020));
        assert_eq!((dec1 / dec2).split(), (700, 0.5.into()));
    }
}
