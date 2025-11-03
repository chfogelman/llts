use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug)]
pub struct Arithmetic<T, Tag>(T, core::marker::PhantomData<Tag>);

impl<T, Tag> Arithmetic<T, Tag> {
    pub fn new(value: T) -> Self {
        value.into()
    }

    pub fn get(self) -> T {
        self.0
    }
}

impl<T, Tag> From<T> for Arithmetic<T, Tag> {
    fn from(value: T) -> Self {
        Self(value, core::marker::PhantomData)
    }
}

impl<T, Tag> Add<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: Add<T, Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Arithmetic<T, Tag>) -> Self::Output {
        (self.0 + rhs.0).into()
    }
}

impl<T, Tag> Sub<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: Sub<T, Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Arithmetic<T, Tag>) -> Self::Output {
        (self.0 - rhs.0).into()
    }
}

impl<T, Tag> Mul<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: Mul<T, Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Arithmetic<T, Tag>) -> Self::Output {
        (self.0 * rhs.0).into()
    }
}

impl<T, Tag> Div<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: Div<T, Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Arithmetic<T, Tag>) -> Self::Output {
        (self.0 / rhs.0).into()
    }
}

impl<T, Tag> Neg for Arithmetic<T, Tag>
where
    T: Neg<Output = T>,
{
    type Output = Self;

    fn neg(self) -> Self::Output {
        (-self.0).into()
    }
}

impl<T, Tag> AddAssign<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: AddAssign<T>,
{
    fn add_assign(&mut self, rhs: Arithmetic<T, Tag>) {
        self.0.add_assign(rhs.0)
    }
}

impl<T, Tag> SubAssign<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: SubAssign<T>,
{
    fn sub_assign(&mut self, rhs: Arithmetic<T, Tag>) {
        self.0.sub_assign(rhs.0)
    }
}

impl<T, Tag> MulAssign<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: MulAssign<T>,
{
    fn mul_assign(&mut self, rhs: Arithmetic<T, Tag>) {
        self.0.mul_assign(rhs.0)
    }
}

impl<T, Tag> DivAssign<Arithmetic<T, Tag>> for Arithmetic<T, Tag>
where
    T: DivAssign<T>,
{
    fn div_assign(&mut self, rhs: Arithmetic<T, Tag>) {
        self.0.div_assign(rhs.0)
    }
}
