use crate::perms::*;
use crate::values::*;
use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
pub struct OwnedValue<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag>,
}
impl<'tag, T: Value> OwnedValue<'tag, T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        unsafe {
            let data = alloc(layout) as *mut T;
            std::ptr::write(data, value);
            OwnedValue {
                pointer: Pointer {
                    _tag: Tag(PhantomData),
                    data: data,
                },
                permission: Active(PhantomData),
            }
        }
    }
    pub fn read(&self) -> T {
        self.pointer.read(&self.permission)
    }
    pub fn write(&mut self, value: T) {
        self.pointer.write(&self.permission, value);
    }

    pub fn borrow(&self, f: impl for<'retag> FnOnce(Ref<'retag, T>)) {
        let immutable = Ref {
            permission: Frozen(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable);
    }
    pub fn borrow_mut(&self, f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag>)) {
        let immutable = RefReserved {
            permission: Reserved(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable, Token(PhantomData));
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
pub struct Ref<'tag, T: Value> {
    pub(crate) pointer: Pointer<'tag, T>,
    pub(crate) permission: Frozen<'tag>,
}

impl<'tag, T: Value> Ref<'tag, T> {
    pub fn read(&self) -> T {
        self.pointer.read(&self.permission)
    }
    pub fn borrow(&self, f: impl for<'retag> FnOnce(Ref<'retag, T>)) {
        let immutable = Ref {
            permission: Frozen(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable);
    }
}

pub struct RefReserved<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Reserved<'tag>,
}

impl<'tag, T: Value> RefReserved<'tag, T> {
    pub fn activate(self, _token: Token<'tag>) -> RefMut<'tag, T> {
        RefMut {
            permission: Active(PhantomData),
            pointer: self.pointer,
            _token,
        }
    }
    pub fn read(&self) -> T {
        self.pointer.read(&self.permission)
    }
    pub fn borrow_mut(
        &self,
        token: Token<'tag>,
        f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag>),
    ) -> Token<'tag> {
        let immutable = RefReserved {
            permission: Reserved(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable, Token(PhantomData));
        token
    }
}
impl<'tag, T: Value> RefMut<'tag, T> {
    pub fn read(&self) -> T {
        self.pointer.read(&self.permission)
    }
    pub fn write(&self, value: T) {
        self.pointer.write(&self.permission, value);
    }
    pub fn borrow_mut(
        self,
        f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag>),
    ) -> Self {
        let immutable = RefReserved {
            permission: Reserved(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable, Token(PhantomData));
        self
    }
}

pub struct RefMut<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag>,
    _token: Token<'tag>,
}

#[cfg(test)]
mod tests {
    use crate::refs::*;
    #[test]
    fn read_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow(|r| {
            assert!(r.read() == 1);
            assert!(r.read() == value.read());
        });
    }

    #[test]
    fn write_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow_mut(|ptr, token| {
            assert!(ptr.read() == 1);
            let ptr_mut = ptr.activate(token);
            ptr_mut.write(2);
            assert!(ptr_mut.read() == 2);
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
        /* Reserved */
        value.borrow_mut(|r1, token1| {
            /* Reserved */
            r1.borrow_mut(token1, |r2, token2| {
                /* We allow foreign reads */
                assert!(r1.read() == r2.read());
                let r2_mut = r2.activate(token2);
                r2_mut.write(2);
                assert!(r2_mut.read() == 2);

                /* BUT
                   we cannot allow multiple writes to occur.
                   if I consume token2 that activate the borrow,
                   I need to ensure that token1 is destroyed.
                */
            });
        });
    }

    #[test]
    fn incompatible_tokens() {
        let value = OwnedValue::new(1);
        value.borrow_mut(|r1, token1| {
            let activated = r1.activate(token1);
            activated.borrow_mut(|r2, token2| {
                r2.activate(token2).write(3);
            });
        });
    }
}
