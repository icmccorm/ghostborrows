use crate::perms::*;
pub trait Value: Copy + Clone + Sized {}
impl Value for i32 {}
impl Value for bool {}

#[derive(Copy, Clone)]
pub struct Pointer<'tag, V: Value> {
    pub(crate) _tag: Tag<'tag, V>,
    pub(crate) data: *mut V,
}

impl<'tag, T: Value> Pointer<'tag, T> {
    pub fn read(&self, _perm: &dyn AllowsRead<'tag>) -> T {
        unsafe { *self.data }
    }
    pub fn write(&self, _perm: &dyn AllowsWrite<'tag>, value: T) {
        unsafe { *self.data = value }
    }
}
