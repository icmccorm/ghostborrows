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
    fn retag<'retag>(&self, tag: Tag<'retag, T>) -> Pointer<'retag, T>
    where
        'tag: 'retag,
    {
        Pointer {
            data: self.data,
            tag,
        }
    }
}

trait Readable<'tag, T: Value> {
    fn read(&self) -> T;
    fn borrow<'retag>(&'retag self) -> Ref<'retag, T>
    where
        'tag: 'retag;
}

trait Writeable<'tag, T: Value>: Readable<'tag, T> {
    fn write(&self, value: T);
    fn borrow_mut<'retag>(&'retag mut self) -> RefMut<'retag, T>
    where
        'tag: 'retag;
}

macro_rules! impl_readable {
    ($type:ty) => {
        impl<'tag, T: Value> Readable<'tag, T> for $type {
            fn read(&self) -> T {
                self.pointer.read(&self.permission)
            }

            fn borrow<'retag>(&self) -> Ref<'retag, T>
            where
                'tag: 'retag,
            {
                Ref {
                    permission: Frozen(PhantomData),
                    pointer: self.pointer.retag(Tag(PhantomData)),
                }
            }
        }
    };
}
macro_rules! impl_writeable {
    ($type:ty) => {
        impl_readable!($type);
        impl<'tag, T: Value> Writeable<'tag, T> for $type {
            fn write(&self, value: T) {
                self.pointer.write(&self.permission, value)
            }

            fn borrow_mut<'retag>(&mut self) -> RefMut<'retag, T>
            where
                'tag: 'retag,
            {
                RefMut {
                    permission: Active(PhantomData),
                    pointer: self.pointer.retag(Tag(PhantomData)),
                }
            }
        }
    };
}

struct OwnedValue<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag, T>,
}
impl_writeable!(OwnedValue<'tag, T>);

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

#[derive(Copy, Clone)]
struct Ref<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Frozen<'tag, T>,
}

impl_readable!(Ref<'tag, T>);

struct RefMut<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag, T>,
}
impl_writeable!(RefMut<'tag, T>);

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn create() {
        let _ = OwnedValue::new(1);
    }

    #[test]
    fn create_ref() {
        let value = OwnedValue::new(1);
        let _ = value.borrow();
    }

    #[test]
    fn read_from_ref() {
        let value = OwnedValue::new(1);
        let reference = value.borrow();
        assert!(reference.read() == 1);
        assert!(reference.read() == value.read());
    }

    #[test]
    fn create_ref_mut() {
        let mut value = OwnedValue::new(1);
        let _ = value.borrow_mut();
    }
    #[test]
    fn write_from_ref_mut() {
        let mut value = OwnedValue::new(1);
        let writeable = value.borrow_mut();
        writeable.write(0);
        assert!(writeable.read() == 0);
        assert!(value.read() == 0);
    }

    #[test]
    fn can_create_and_use_multiple_refs() {
        let value = OwnedValue::new(1);
        let ref_1 = value.borrow();
        let ref_2 = value.borrow();
        let ref_3 = value.borrow();
        assert!(ref_1.read() == ref_2.read());
        assert!(ref_2.read() == ref_3.read());
        assert!(ref_3.read() == value.read());
    }

    #[test]
    fn immutable_reborrow() {
        let mut value = OwnedValue::new(1);
        let ref_1 = value.borrow();
        let ref_1_1 = ref_1.borrow();
        assert!(ref_1_1.read() == ref_1.read());
        assert!(ref_1_1.read() == value.read());
        
        let ref_mut_1 = value.borrow_mut();
        let ref_mut_1_1 = ref_mut_1.borrow();
        assert!(ref_mut_1_1.read() == value.read());
    }

    #[test]
    fn mutable_reborrow() {
        let mut value = OwnedValue::new(1);
        /*
            Limitation: since we want the borrow checker
            to statically prevent aliasing violations, we have
            to declare each `RefMut` as 'mut', even if we're not
            actually mutating the reference itself. 

            This is due to the signature of `borrow_mut`:
            ```
            fn borrow_mut<'retag>(&'retag mut self) 
            ```
            We must receive &mut, which requires the borrowed
            location to be mutable, in order to statically enforce
            aliasing XOR mutability. RefCell implements a similar method
            that takes &self, but it only works because RefCell checks
            borrow rules dynamically. 

            If Rust had a construct for a unique, read-only reference, then
            we could implement this.
         */
        let mut ref_mut_1 = value.borrow_mut();
        let ref_mut_1_1 = ref_mut_1.borrow_mut();
        ref_mut_1_1.write(2);
        assert!(ref_mut_1_1.read() == value.read())
    }
}
