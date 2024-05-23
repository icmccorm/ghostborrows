pub mod perms;
pub mod refs;

pub use perms::*;
pub use refs::*;

#[macro_export]
macro_rules! split_mut {
    ($x:expr) => {
        RefMut::from($x).split()
    };
}

#[macro_export]
macro_rules! split {
    ($x:expr) => {
        Ref::from($x).split()
    };
}
