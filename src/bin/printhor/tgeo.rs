//! A computing geometry Q&D API to make facilitate vector operations and provide numerically stability (undefs, etc).
//! It implies a cost, of course
#[allow(unused)]
use crate::hwa;
#[cfg(not(feature = "native"))]
use crate::alloc::string::ToString;
use crate::math::Real;
use num_traits::float::FloatCore;
use bitflags::bitflags;

bitflags! {
    #[derive(PartialEq, Eq)]
    pub struct CoordSel: u8 {
        const X = 0b00000001;
        const Y = 0b00000010;
        const Z = 0b00000100;
        const E = 0b00001000;
        const XYZ = Self::X.bits() | Self::Y.bits() | Self::Z.bits();
        const XYZE = Self::X.bits() | Self::Y.bits() | Self::Z.bits() | Self::E.bits();
    }
}


pub trait ArithmeticOps: Copy
    + Clone
    + core::ops::Add<Self, Output=Self>
    + core::ops::Mul<Self, Output=Self>
    + core::cmp::PartialEq<Self>
    + core::cmp::PartialOrd<Self>
{
    fn zero() -> Self;
    fn one() -> Self;
    fn is_zero(&self) -> bool;
    fn abs(&self) -> Self;
}

pub trait RealOps
{
    fn pow(&self, power: i32) -> Self;
    fn sqrt(&self) -> Option<Self> where Self: Sized;
    fn rdp(&self, digits: u32) -> Self;
}

#[derive(Copy, Clone)]
pub struct TVector<T>
    where T: ArithmeticOps
{
    pub x: Option<T>,
    pub y: Option<T>,
    pub z: Option<T>,
    pub e: Option<T>,
}

#[allow(unused)]
impl<T> TVector<T>
where T: ArithmeticOps
{
    #[inline]
    pub const fn new() -> Self {
        Self::nan()
    }

    #[inline]
    pub const fn from_coords(x: Option<T>, y: Option<T>, z: Option<T>, e: Option<T>) -> Self {
        Self {
            x,y,z,e
        }
    }

    #[inline]
    pub fn map_coords<U, F>(&self, f: F) -> TVector<U>
    where F: Fn(T) -> Option<U>, U: ArithmeticOps
    {
        TVector {
            x: self.x.and_then(|v| f(v)),
            y: self.y.and_then(|v| f(v)),
            z: self.z.and_then(|v| f(v)),
            e: self.e.and_then(|v| f(v)),
        }
    }

    #[inline]
    pub fn map_coord<F>(&self, coord_idx: CoordSel, f: F) -> TVector<T>
        where F: Fn(T, CoordSel) -> Option<T>, T: ArithmeticOps
    {
        TVector {
            x: if coord_idx.contains(CoordSel::X) { self.x.and_then(|v| f(v, CoordSel::X)) } else { self.x },
            y: if coord_idx.contains(CoordSel::Y) { self.y.and_then(|v| f(v, CoordSel::Y)) } else { self.y },
            z: if coord_idx.contains(CoordSel::Z) { self.z.and_then(|v| f(v, CoordSel::Z)) } else { self.z },
            e: if coord_idx.contains(CoordSel::E) { self.e.and_then(|v| f(v, CoordSel::E)) } else { self.e },
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn with_coord(&self, coord_idx: CoordSel, val: Option<T>) -> Self {
        Self{
            x: if coord_idx.contains(CoordSel::X) { val } else { self.x },
            y: if coord_idx.contains(CoordSel::Y) { val } else { self.y },
            z: if coord_idx.contains(CoordSel::Z) { val } else { self.z },
            e: if coord_idx.contains(CoordSel::E) { val } else { self.e },
        }
    }

    #[inline]
    #[allow(unused)]
    pub fn set_coord(&mut self, coord_idx: CoordSel, val: Option<T>) -> &Self {
        if coord_idx.contains(CoordSel::X) { self.x = val }
        if coord_idx.contains(CoordSel::Y) { self.y = val }
        if coord_idx.contains(CoordSel::Z) { self.z = val }
        if coord_idx.contains(CoordSel::E) { self.e = val }
        self
    }

    #[inline]
    #[allow(unused)]
    pub fn assign(&mut self, coord_idx: CoordSel, other: &Self) -> &Self {
        if coord_idx.contains(CoordSel::X) { self.x = other.x }
        if coord_idx.contains(CoordSel::Y) { self.y = other.y }
        if coord_idx.contains(CoordSel::Z) { self.z = other.z }
        if coord_idx.contains(CoordSel::E) { self.e = other.e }
        self
    }

    #[inline]
    #[allow(unused)]
    pub fn assign_if_set(&mut self, coord_idx: CoordSel, other: &Self) -> &Self {
        if coord_idx.contains(CoordSel::X) && other.x.is_some() { self.x = other.x }
        if coord_idx.contains(CoordSel::Y) && other.y.is_some() { self.y = other.y }
        if coord_idx.contains(CoordSel::Z) && other.z.is_some() { self.z = other.z }
        if coord_idx.contains(CoordSel::E) && other.e.is_some() { self.e = other.e }
        self
    }

    #[inline]
    #[allow(unused)]
    pub fn with_coord_if_set(&self, coord_idx: CoordSel, val: Option<T>) -> Self {
        Self {
            x: if coord_idx.contains(CoordSel::X) && self.x.is_some() { val } else { self.x },
            y: if coord_idx.contains(CoordSel::Y) && self.y.is_some() { val } else { self.y },
            z: if coord_idx.contains(CoordSel::Z) && self.z.is_some() { val } else { self.z },
            e: if coord_idx.contains(CoordSel::E) && self.e.is_some() { val } else { self.e },
        }
    }

    pub fn clamp_coord(&self, coord_idx: CoordSel, upper_bound: T) -> Self
    where T: core::cmp::PartialOrd<T>
    {
        Self {
            x: if coord_idx.contains(CoordSel::X) { self.x.and_then(|v| if v > upper_bound {Some(v)} else {Some(upper_bound)})} else {self.x},
            y: if coord_idx.contains(CoordSel::Y) { self.y.and_then(|v| if v > upper_bound {Some(v)} else {Some(upper_bound)})} else {self.y},
            z: if coord_idx.contains(CoordSel::Z) { self.z.and_then(|v| if v > upper_bound {Some(v)} else {Some(upper_bound)})} else {self.z},
            e: if coord_idx.contains(CoordSel::E) { self.e.and_then(|v| if v > upper_bound {Some(v)} else {Some(upper_bound)})} else {self.e},
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn clamp(&self, rhs: TVector<T>) -> TVector<T> {
        TVector {
            x: match self.x {
                None => None,
                Some(lv) => match rhs.x {
                    None => Some(lv),
                    Some(rv) => match lv > rv {
                        true => Some(rv),
                        false => Some(lv),
                    }
                }
            },
            y: match self.y {
                None => None,
                Some(lv) => match rhs.y {
                    None => Some(lv),
                    Some(rv) => match lv > rv {
                        true => Some(rv),
                        false => Some(lv),
                    }
                }
            },
            z: match self.z {
                None => None,
                Some(lv) => match rhs.z {
                    None => Some(lv),
                    Some(rv) => match lv > rv {
                        true => Some(rv),
                        false => Some(lv),
                    }
                }
            },
            e: match self.e {
                None => None,
                Some(lv) => match rhs.e {
                    None => Some(lv),
                    Some(rv) => match lv > rv {
                        true => Some(rv),
                        false => Some(lv),
                    }
                }
            },
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn max(&self) -> Option<T> {
        let mut m: Option<T> = None;
        if let Some(x) = self.x {
            m = Some(x);
        }
        if let Some(y) = self.y {
            if let Some(mr) = m {
                if mr.lt(&y) {
                    m = Some(y);
                }
            }
            else {
                m = Some(y);
            }
        }
        if let Some(z) = self.z {
            if let Some(mr) = m {
                if mr.lt(&z) {
                    m = Some(z);
                }
            }
            else {
                m = Some(z);
            }
        }
        if let Some(e) = self.e {
            if let Some(mr) = m {
                if mr.lt(&e) {
                    m = Some(e);
                }
            }
            else {
                m = Some(e);
            }
        }
        m
    }

    #[allow(unused)]
    #[inline]
    pub fn min(&self) -> Option<T> {
        let mut m: Option<T> = None;
        if let Some(x) = self.x {
            m = Some(x);
        }
        if let Some(y) = self.y {
            if let Some(mr) = m {
                if mr.gt(&y) {
                    m = Some(y);
                }
            }
            else {
                m = Some(y);
            }
        }
        if let Some(z) = self.z {
            if let Some(mr) = m {
                if mr.gt(&z) {
                    m = Some(z);
                }
            }
            else {
                m = Some(z);
            }
        }
        if let Some(e) = self.e {
            if let Some(mr) = m {
                if mr.gt(&e) {
                    m = Some(e);
                }
            }
            else {
                m = Some(e);
            }
        }
        m
    }

    #[inline]
    pub const fn nan() -> Self {
        Self {
            x: None,
            y: None,
            z: None,
            e: None,
        }
    }

    #[allow(unused)]
    #[inline]
    pub fn zero() -> Self {
        Self {
            x: Some(T::zero()),
            y: Some(T::zero()),
            z: Some(T::zero()),
            e: Some(T::zero()),
        }
    }

    #[inline]
    pub fn one() -> Self {
        Self {
            x: Some(T::one()),
            y: Some(T::one()),
            z: Some(T::one()),
            e: Some(T::one()),
        }
    }

    pub fn map_nan(&self, value: T) -> Self {
        Self {
            x: self.x.map_or_else(|| Some(value), |cv| Some(cv)),
            y: self.y.map_or_else(|| Some(value), |cv| Some(cv)),
            z: self.z.map_or_else(|| Some(value), |cv| Some(cv)),
            e: self.e.map_or_else(|| Some(value), |cv| Some(cv)),
        }
    }

    pub fn map_val(&self, value: T) -> Self {
        Self {
            x: self.x.and(Some(value)),
            y: self.y.and(Some(value)),
            z: self.z.and(Some(value)),
            e: self.e.and(Some(value)),
        }
    }

    pub fn sum(&self) -> T
        where T: core::ops::Add<T, Output=T>
    {
        let x = self.x.unwrap_or(T::zero());
        let y = self.y.unwrap_or(T::zero());
        let z = self.z.unwrap_or(T::zero());
        let e = self.e.unwrap_or(T::zero());
        x + y + z + e
    }

    pub fn abs(&self) -> Self
    {
        Self {
            x: self.x.and_then(|v| Some(v.abs())),
            y: self.y.and_then(|v| Some(v.abs())),
            z: self.z.and_then(|v| Some(v.abs())),
            e: self.e.and_then(|v| Some(v.abs())),
        }
    }

}

impl<T> TVector<T>
    where T: ArithmeticOps + RealOps,
          TVector<T>: core::ops::Div<T, Output = TVector<T>>
          + core::ops::Div<TVector<T>, Output = TVector<T>>

{
    pub fn pow(&self, power: i32) -> Self {
        Self {
            x: self.x.map_or_else(|| None, |v| Some(v.pow(power))),
            y: self.y.map_or_else(|| None, |v| Some(v.pow(power))),
            z: self.z.map_or_else(|| None, |v| Some(v.pow(power))),
            e: self.e.map_or_else(|| None, |v| Some(v.pow(power))),
        }
    }

    #[allow(unused)]
    pub fn sqrt(&self) -> Self {
        Self {
            x: self.x.map_or_else(|| None, |v| v.sqrt()),
            y: self.y.map_or_else(|| None, |v| v.sqrt()),
            z: self.z.map_or_else(|| None, |v| v.sqrt()),
            e: self.e.map_or_else(|| None, |v| v.sqrt()),
        }
    }

    pub fn norm2(&self) -> Option<T>
    where T: RealOps
    {
        self
            .pow(2)
            .sum()
            .sqrt()
    }

    #[allow(unused)]
    pub fn unit(&self) -> Self
    {
        match self.norm2() {
            None => Self::nan(),
            Some(norm) => match norm.is_zero() {
                true => Self::nan(),
                false => {
                    let t1 = *self;
                    let t2 = norm;
                    t1 / t2
                }
            }
        }
    }
    /***
    custom behavior
     */
    #[allow(unused)]
    pub fn decompose_normal(&self) -> (Self, T)
    where TVector<T>: core::ops::Div<T, Output=TVector<T>>, T: ArithmeticOps + RealOps
    {
        match self.norm2() {
            None => (Self::nan(), T::zero()),
            Some(norm) => match norm.is_zero() {
                true => (Self::nan(), T::zero()),
                false => {
                    ((*self) / norm.clone(), norm)
                }
            }
        }
    }

    #[allow(unused)]
    pub fn rdp(&self, digits: u32) -> TVector<T> {
        Self {
            x: self.x.map_or_else(|| None, |v| Some(v.rdp(digits))),
            y: self.y.map_or_else(|| None, |v| Some(v.rdp(digits))),
            z: self.z.map_or_else(|| None, |v| Some(v.rdp(digits))),
            e: self.e.map_or_else(|| None, |v| Some(v.rdp(digits))),
        }
    }


}

impl<T> Default for TVector<T>
where T: ArithmeticOps
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> core::fmt::Display for TVector<T>
where T: ArithmeticOps + ToString
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut spacing = false;
        if let Some(v) = &self.x {
            core::write!(f, "X {}", v.to_string())?;
            spacing = true;
        }
        if let Some(v) = &self.y {
            core::write!(f, "{}Y {}", if spacing {" "} else {""}, v.to_string())?;
            spacing = true;
        }
        if let Some(v) = &self.z {
            core::write!(f, "{}Z {}", if spacing {" "} else {""}, v.to_string())?;
            spacing = true;
        }
        if let Some(v) = &self.e {
            core::write!(f, "{}E {}", if spacing {" "} else {""}, v.to_string())?;
        }
        Ok(())
    }
}

#[cfg(feature = "with-defmt")]
impl<T> hwa::defmt::Format for TVector<T>
where T: ArithmeticOps + ToString
{
    fn format(&self, fmt: hwa::defmt::Formatter) {

        let mut spacing = false;
        if let Some(v) = &self.x {
            hwa::defmt::write!(fmt, "X {}", v.to_string().as_str());
            spacing = true;
        }
        if let Some(v) = &self.y {
            hwa::defmt::write!(fmt, "{}Y {}", if spacing {" "} else {""}, v.to_string().as_str());
            spacing = true;
        }
        if let Some(v) = &self.z {
            hwa::defmt::write!(fmt, "{}Z {}", if spacing {" "} else {""}, v.to_string().as_str());
            spacing = true;
        }
        if let Some(v) = &self.e {
            hwa::defmt::write!(fmt, "{}E {}", if spacing {" "} else {""}, v.to_string().as_str());
        }
    }
}


impl<T> core::ops::Add for TVector<T>
where T: ArithmeticOps + core::ops::Add<Output=T>
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x.and_then(|x0| rhs.x.and_then(|x1| Some(x0 + x1))),
            y: self.y.and_then(|y0| rhs.y.and_then(|y1| Some(y0 + y1))),
            z: self.z.and_then(|z0| rhs.z.and_then(|z1| Some(z0 + z1))),
            e: self.e.and_then(|e0| rhs.e.and_then(|e1| Some(e0 + e1))),
        }

    }
}

impl<T> core::ops::AddAssign for TVector<T>
    where T: ArithmeticOps + core::ops::AddAssign
{

    fn add_assign(&mut self, rhs: Self) {
        self.x.as_mut().map(|x0| rhs.x.map(|x1| (*x0) += x1));
        self.y.as_mut().map(|y0| rhs.y.map(|y1| (*y0) += y1));
        self.z.as_mut().map(|z0| rhs.z.map(|z1| (*z0) += z1));
        self.e.as_mut().map(|e0| rhs.e.map(|e1| (*e0) += e1));
    }
}

impl<T> core::ops::Sub for TVector<T>
    where T: ArithmeticOps + core::ops::Sub<Output=T>
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            x: self.x.and_then(|x0| rhs.x.and_then(|x1| Some(x0 - x1))),
            y: self.y.and_then(|y0| rhs.y.and_then(|y1| Some(y0 - y1))),
            z: self.z.and_then(|z0| rhs.z.and_then(|z1| Some(z0 - z1))),
            e: self.e.and_then(|e0| rhs.e.and_then(|e1| Some(e0 - e1))),
        }
    }
}

impl<T> core::ops::SubAssign for TVector<T>
    where T: ArithmeticOps + core::ops::SubAssign
{

    fn sub_assign(&mut self, rhs: Self) {
        self.x.as_mut().map(|x0| rhs.x.map(|x1| (*x0) -= x1));
        self.y.as_mut().map(|y0| rhs.y.map(|y1| (*y0) -= y1));
        self.z.as_mut().map(|z0| rhs.z.map(|z1| (*z0) -= z1));
        self.e.as_mut().map(|e0| rhs.e.map(|e1| (*e0) -= e1));
    }
}

impl<T> core::ops::Mul<TVector<T>> for TVector<T>
    where T: ArithmeticOps + core::ops::Mul<T, Output=T>
{
    type Output = Self;

    fn mul(self, rhs: TVector<T>) -> Self::Output {
        Self::Output {
            x: self.x.and_then(|x0| rhs.x.and_then(|x1| Some(x0 * x1))),
            y: self.y.and_then(|y0| rhs.y.and_then(|y1| Some(y0 * y1))),
            z: self.z.and_then(|z0| rhs.z.and_then(|z1| Some(z0 * z1))),
            e: self.e.and_then(|e0| rhs.e.and_then(|e1| Some(e0 * e1))),
        }

    }
}

impl<T> core::ops::Mul<T> for TVector<T>
    where T: ArithmeticOps + core::ops::Mul<T, Output=T>
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self::Output {
            x: self.x.and_then(|v| Some(v * rhs)),
            y: self.y.and_then(|v| Some(v * rhs)),
            z: self.z.and_then(|v| Some(v * rhs)),
            e: self.e.and_then(|v| Some(v * rhs)),
        }

    }
}

impl<T> core::ops::Div<TVector<T>> for TVector<T>
    where T: ArithmeticOps + RealOps + core::ops::Div<T, Output=T>
{
    type Output = TVector<T>;

    fn div(self, rhs: TVector<T>) -> Self::Output
    {
        Self::Output {
            x: self.x.map_or_else(|| None, |v| {
                rhs.x.map_or_else(|| None, |divisor| {
                    if !divisor.is_zero() { Some(v.div(divisor)) }
                    else { None }
                })
            }),
            y: self.y.map_or_else(|| None, |v| {
                rhs.y.map_or_else(|| None, |divisor| {
                    if !divisor.is_zero() { Some(v.div(divisor)) }
                    else { None }
                })
            }),
            z: self.z.map_or_else(|| None, |v| {
                rhs.z.map_or_else(|| None, |divisor| {
                    if !divisor.is_zero() { Some(v.div(divisor)) }
                    else { None }
                })
            }),
            e: self.e.map_or_else(|| None, |v| {
                rhs.e.map_or_else(|| None, |divisor| {
                    if !divisor.is_zero() { Some(v.div(divisor)) }
                    else { None }
                })
            }),
        }
    }
}

impl<T> core::ops::Div<T> for TVector<T>
    where T: ArithmeticOps + RealOps + core::ops::Div<T, Output=T>
{
    type Output = TVector<T>;

    fn div(self, rhs: T) -> Self::Output
    {
        if rhs.is_zero() {
            Self::Output::nan()
        }
        else {
            Self::Output {
                x: self.x.and_then(|x0| Some(x0 / rhs)),
                y: self.y.and_then(|y0| Some(y0 / rhs)),
                z: self.z.and_then(|z0| Some(z0 / rhs)),
                e: self.e.and_then(|e0| Some(e0 / rhs)),
            }
        }
    }
}

//////////////

impl ArithmeticOps for i32 {
    #[inline]
    fn zero() -> Self {
        0
    }
    #[inline]
    fn one() -> Self {
        1
    }

    fn is_zero(&self) -> bool
    where Self: core::cmp::PartialEq
    {
        self.eq(&0)
    }

    fn abs(&self) -> Self {
        i32::abs(*self)
    }
}

impl ArithmeticOps for u32 {
    #[inline]
    fn zero() -> Self {
        0
    }
    #[inline]
    fn one() -> Self {
        1
    }

    fn is_zero(&self) -> bool
        where Self: core::cmp::PartialEq
    {
        self.eq(&0)
    }

    fn abs(&self) -> Self {
        *self
    }
}

impl ArithmeticOps for u16 {
    #[inline]
    fn zero() -> Self {
        0
    }
    #[inline]
    fn one() -> Self {
        1
    }

    fn is_zero(&self) -> bool
        where Self: core::cmp::PartialEq
    {
        self.eq(&0)
    }

    fn abs(&self) -> Self {
        *self
    }
}

impl ArithmeticOps for u8 {
    #[inline]
    fn zero() -> Self {
        0
    }
    #[inline]
    fn one() -> Self {
        1
    }

    fn is_zero(&self) -> bool
        where Self: core::cmp::PartialEq
    {
        self.eq(&0)
    }

    fn abs(&self) -> Self {
        *self
    }
}

impl ArithmeticOps for f32 {
    #[inline]
    fn zero() -> Self {
        0.0f32
    }
    #[inline]
    fn one() -> Self {
        1.0f32
    }

    fn is_zero(&self) -> bool
        where Self: core::cmp::PartialEq
    {
        self.eq(&0.0f32)
    }

    fn abs(&self) -> Self {
        <f32 as FloatCore>::abs(*self)
    }
}

impl RealOps for f32 {

    fn pow(&self, p: i32) -> Self {
        self.powi(p)
    }

    fn sqrt(&self) -> Option<Self> where Self: Sized {
        if !self.is_sign_negative() {
            Some(micromath::F32(*self).sqrt().0)
            //f32::sqrt(self)
        }
        else {
            None
        }

    }

    fn rdp(&self, digits: u32) -> Self {
        let dd =  10.0f32.powi(digits as i32);
        (self * dd).round() * dd
    }
}


impl ArithmeticOps for Real {
    #[inline]
    fn zero() -> Self {
        Real::zero()
    }

    #[inline]
    fn one() -> Self {
        Real::one()
    }

    fn is_zero(&self) -> bool {
        Real::is_zero(self)
    }

    fn abs(&self) -> Self {
        Real::abs(*self)
    }
}

impl RealOps for Real {

    fn pow(&self, p: i32) -> Real {
        self.powi(p)
    }

    fn sqrt(&self) -> Option<Self> where Self: Sized {
        Real::sqrt(*self)
    }

    fn rdp(&self, digits: u32) -> Self {
        Real::round_dp(self, digits)
    }
}

#[allow(unused)]
pub fn test() {
    let pos: TVector<i32> = TVector::new();
    let p1: TVector<i32> = TVector::one();

    let p0 = pos.map_nan(0);

    let r = p0 + p1;
    crate::hwa::info!("{}", r);
}


