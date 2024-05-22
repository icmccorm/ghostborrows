use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;

pub trait Value: Copy + Clone + Sized {}
impl Value for i32 {}

trait AllowsRead<'tag, T: Value> {}
trait AllowsWrite<'tag, T: Value> {}

/// An invariant lifetime, matching GhostCell
#[derive(Copy, Clone)]
pub struct Tag<'tag, V>(PhantomData<*mut &'tag V>);
//struct Reserved<'tag>(PhantomData<*mut &'tag ()>);

#[derive(Copy, Clone)]
pub struct Frozen<'tag, V>(PhantomData<*mut &'tag V>);
impl<'tag, T: Value> AllowsRead<'tag, T> for Frozen<'tag, T> {}

pub struct Active<'tag, V>(PhantomData<*mut &'tag V>);
impl<'tag, T: Value> AllowsRead<'tag, T> for Active<'tag, T> {}
impl<'tag, T: Value> AllowsWrite<'tag, T> for Active<'tag, T> {}

#[derive(Copy, Clone)]
struct Pointer<'tag, V> {
    tag: Tag<'tag, V>,
    data: *mut V,
}

impl<'tag, T: Value> Pointer<'tag, T> {
    fn read(&self, perm: &dyn AllowsRead<'tag, T>) -> T {
        unsafe { *self.data }
    }
    fn write(&self, perm: &dyn AllowsRead<'tag, T>, value: T) {
        unsafe { *self.data = value }
    }
}

#[derive(Copy, Clone)]
struct Ref<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Frozen<'tag, T>,
}

macro_rules! impl_readable {
    ($type:ident) => {
        impl<'tag, T: Value> Readable<'tag, T> for $type<'tag, T> {
            fn read(&self) -> T {
                self.pointer.read(&self.permission)
            }
            fn borrow<R>(&self, f: impl for<'retag> FnOnce(Ref<'retag, T>) -> R) -> R {
                let immutable = Ref {
                    permission: Frozen(PhantomData),
                    pointer: Pointer {
                        tag: Tag(PhantomData),
                        data: self.pointer.data,
                    },
                };
                f(immutable)
            }
        }
    };
}
impl_readable!(Ref);

trait Readable<'tag, T: Value> {
    fn read(&self) -> T;
    fn borrow<'lt, R>(&self, f: impl for<'retag> FnOnce(Ref<'retag, T>) -> R) -> R;
}

struct RefMut<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag, T>,
}
trait Writeable<'tag, T: Value> {
    fn write(&self, value: T);
    fn borrow_mut<'lt, R>(&self, f: impl for<'retag> FnOnce(RefMut<'retag, T>) -> R) -> R;
}
macro_rules! impl_writeable {
    ($type:ident) => {
        impl<'tag, T: Value> Writeable<'tag, T> for $type<'tag, T> {
            fn write(&self, value: T) {
                self.pointer.write(&self.permission, value)
            }
            fn borrow_mut<R>(&self, f: impl for<'retag> FnOnce(RefMut<'retag, T>) -> R) -> R {
                let immutable = RefMut {
                    permission: Active(PhantomData),
                    pointer: Pointer {
                        tag: Tag(PhantomData),
                        data: self.pointer.data,
                    },
                };
                f(immutable)
            }
        }
    };
}
impl_readable!(RefMut);
impl_writeable!(RefMut);

struct OwnedValue<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag, T>,
}
impl_readable!(OwnedValue);
impl_writeable!(OwnedValue);

impl<'tag, T: Value> OwnedValue<'tag, T> {
    fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        unsafe {
            let data = alloc(layout) as *mut T;
            std::ptr::write(data, value);
            OwnedValue {
                pointer: Pointer {
                    tag: Tag(PhantomData),
                    data: data,
                },
                permission: Active(PhantomData),
            }
        }
    }
}

impl<'tag, T: Value> Drop for OwnedValue<'tag, T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::new::<T>();
            dealloc(self.pointer.data as *mut u8, layout);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    #[test]
    fn read_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow(|r| {
            assert!(r.read() == 1);
            assert!(r.read() == value.read());
        });
    }
    fn write_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow_mut(|r| {
            assert!(r.read() == 1);
            assert!(r.read() == value.read());
            r.write(2);
            assert!(r.read() == 2);
        });
    }

    #[test]
    fn can_create_and_use_multiple_refs() {
        let value = OwnedValue::new(1);
        value.borrow(|r1| {
            value.borrow(|r2| {
                value.borrow(|r3| {
                    assert!(r1.read() == r2.read());
                    assert!(r2.read() == r3.read());
                    assert!(r3.read() == value.read());
                });
            });
        });
    }

    #[test]
    fn immutable_reborrow() {
        let value = OwnedValue::new(1);
        value.borrow(|r1| {
            r1.borrow(|r2| {
                assert!(r1.read() == r2.read());
                assert!(r2.read() == value.read());
            });
        });
    }

    #[test]
    fn mutable_reborrow() {
        let value = OwnedValue::new(1);
        value.borrow_mut(|r1| {
            r1.borrow_mut(|r2| {
                assert!(r1.read() == r2.read());
                assert!(r2.read() == value.read());
                r2.write(2);
                assert!(r2.read() == value.read());
            });
        });
    }
}
