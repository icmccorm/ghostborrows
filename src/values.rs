
use crate::perms::*;

#[derive(Copy, Clone)]
pub struct Pointer<'tag> {
    pub(crate) _tag: Tag<'tag>,
    pub(crate) data: *mut u8,
}

impl<'tag> Pointer<'tag> {
    pub fn read<T>(&self, _perm: &dyn AllowsRead<'tag, T>, f: impl for<'b> FnOnce(&'b T)) {
        f(unsafe { &*(self.data as *mut T)})
    }
    pub fn write<T>(&self, _perm: &dyn AllowsWrite<'tag, T>, f: impl for<'b> FnOnce(&'b mut T)) {
        f(unsafe { &mut *(self.data as *mut T) })
    }
}
