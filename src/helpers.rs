use num_traits::Signed;
// use ultraviolet::{IVec2, UVec2, Vec2};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
// use num_traits::AsPrimitive;

pub fn abs_max<T>(x: T, y: T) -> T
where
    T: PartialOrd + Signed,
{
    if x.abs() > y.abs() { x } else { y }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Action {
    Init,
    DrawLine,
    DrawRectangle,
    DrawCircle,
    #[allow(dead_code)]
    DrawEllipse,
    DrawPolygon,
    Erase,
    Fill,
    #[allow(dead_code)]
    CutRectangle,
}

// macro_rules! impl_from_vec {
//     ($vec_type:ty, $type:ty) => {
//         impl From<$vec_type> for $type {
//             fn from(vec: $vec_type) -> Self {
//                 Self::new(vec.x, vec.y)
//             }
//         }
//     };
// }

#[derive(Clone, Copy, Default)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    pub const fn new(width: T, height: T) -> Self {
        Self {
            width,
            height,
        }
    }
}

// TODO
// impl<T, U> From<Size<T>> for Size<U>
// where
//     T: AsPrimitive<U> + Copy + 'static,
//     U: Copy + 'static,
// {
//     fn from(size: Size<T>) -> Self {
//         Size {
//             width: size.width.as_(),
//             height: size.height.as_(),
//         }
//     }
// }

impl<T, U> From<PhysicalSize<T>> for Size<U>
where
    U: From<T>,
{
    fn from(size: PhysicalSize<T>) -> Self {
        Self::new(size.width.into(), size.height.into())
    }
}

impl<T, U> From<LogicalSize<T>> for Size<U>
where
    U: From<T>,
{
    fn from(size: LogicalSize<T>) -> Self {
        Self::new(size.width.into(), size.height.into())
    }
}

// impl_from_vec!(IVec2, Size<i32>);
// impl_from_vec!(UVec2, Size<u32>);
// impl_from_vec!(Vec2, Size<f32>);

#[derive(Clone, Copy, Default)]
pub struct Position<T> {
    pub x: T,
    pub y: T,
}

impl<T> Position<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self {
            x,
            y,
        }
    }
}

impl<T, U> From<PhysicalPosition<T>> for Position<U>
where
    U: From<T>,
{
    fn from(position: PhysicalPosition<T>) -> Self {
        Self::new(position.x.into(), position.y.into())
    }
}

impl<T, U> From<LogicalPosition<T>> for Position<U>
where
    U: From<T>,
{
    fn from(position: LogicalPosition<T>) -> Self {
        Self::new(position.x.into(), position.y.into())
    }
}

// impl_from_vec!(IVec2, Position<i32>);
// impl_from_vec!(UVec2, Position<u32>);
// impl_from_vec!(Vec2, Position<f32>);
