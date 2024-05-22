use crate::perms::*;
use crate::values::*;
use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;

pub struct OwnedValue<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Active<'tag, T>,
    _dealloc: Dealloc<'tag, T>,
}

impl<'tag, T> OwnedValue<'tag, T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        unsafe {
            let data = alloc(layout);
            std::ptr::write(data as *mut T, value);
            OwnedValue {
                pointer: Pointer {
                    _tag: Tag(PhantomData),
                    data: data,
                },
                permission: Active(PhantomData),
                _dealloc: Dealloc(PhantomData),
            }
        }
    }
    pub fn into_raw(self) -> (Pointer<'tag>, Active<'tag, T>, Dealloc<'tag, T>) {
        let pointer = Pointer {
            _tag: Tag(PhantomData),
            data: self.pointer.data,
        };
        let permission = Active(PhantomData);
        let dealloc = Dealloc(PhantomData);
        std::mem::forget(self);
        (pointer, permission, dealloc)
    }

    pub fn from_raw(
        pointer: Pointer<'tag>,
        permission: Active<'tag, T>,
        _dealloc: Dealloc<'tag, T>,
    ) -> Self {
        OwnedValue {
            pointer,
            permission,
            _dealloc,
        }
    }

    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
    }
    pub fn write(&mut self, f: impl for<'b> FnOnce(&'b mut T)) {
        self.pointer.write(&self.permission, f);
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

impl<'tag, T> Drop for OwnedValue<'tag, T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::new::<T>();
            dealloc(self.pointer.data as *mut u8, layout);
        }
    }
}

#[derive(Copy, Clone)]
pub struct Ref<'tag, T> {
    pub(crate) pointer: Pointer<'tag>,
    pub(crate) permission: Frozen<'tag, T>,
}

impl<'tag, T> Ref<'tag, T> {
    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
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

pub struct RefReserved<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Reserved<'tag, T>,
}

impl<'tag, T> RefReserved<'tag, T> {
    pub fn activate(self, _token: Token<'tag>) -> RefMut<'tag, T> {
        RefMut {
            permission: Active(PhantomData),
            pointer: self.pointer,
            _token,
        }
    }
    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
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
impl<'tag, T> RefMut<'tag, T> {
    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
    }
    pub fn write(&self, f: impl for<'b> FnOnce(&'b mut T)) {
        self.pointer.write(&self.permission, f)
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

pub struct RefMut<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Active<'tag, T>,
    _token: Token<'tag>,
}

#[cfg(test)]
mod tests {
    use crate::refs::*;
    #[test]
    fn read_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow(|r| {
            r.read(|b| {
                assert!(*b == 1);
            });
            r.read(|br| {
                value.read(|bv| {
                    assert!(*br == *bv);
                });
            });
        });
    }

    #[test]
    fn write_from_ref() {
        let value = OwnedValue::new(1);
        value.borrow_mut(|ptr, token| {
            ptr.read(|b| {
                assert!(*b == 1);
            });
            let ptr_mut = ptr.activate(token);
            ptr_mut.write(|bp| *bp = 2);
            ptr_mut.read(|b| {
                assert!(*b == 2);
            });
        });
    }

    #[test]
    fn can_create_and_use_multiple_refs() {
        let value = OwnedValue::new(1);
        value.borrow(|r1| {
            value.borrow(|r2| {
                value.borrow(|r3| {
                    r1.read(|r1| {
                        r2.read(|r2| {
                            r3.read(|r3| {
                                assert!(*r1 == *r2);
                                assert!(*r2 == *r3);
                                assert!(*r3 == 1);
                            });
                        });
                    });
                });
            });
        });
    }

    #[test]
    fn immutable_reborrow() {
        let value = OwnedValue::new(1);
        value.borrow(|r1| {
            r1.borrow(|r2| {
                r1.read(|r1| {
                    r2.read(|r2| {
                        assert!(*r1 == *r2);
                    });
                });
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
                r1.read(|r1| {
                    r2.read(|r2| {
                        assert!(*r1 == *r2);
                    });
                });

                let r2_mut = r2.activate(token2);
                r2_mut.write(|rm| *rm = 2);
                r2_mut.read(|rm| {
                    assert!(*rm == 2);
                });

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
                r2.activate(token2).write(|r2m| *r2m = 3);
            });
        });
    }
}
