use crate::perms::*;
use crate::values::*;
use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;

pub struct Value<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Write<'tag, T>,
    _dealloc: Dealloc<'tag, T>,
}

impl<'tag, T> Value<'tag, T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        unsafe {
            let data = alloc(layout);
            std::ptr::write(data as *mut T, value);
            Value {
                pointer: Pointer {
                    _tag: Tag(PhantomData),
                    data,
                },
                permission: Write(Token(PhantomData)),
                _dealloc: Dealloc(PhantomData),
            }
        }
    }
    pub fn into_raw(self) -> (Pointer<'tag>, Write<'tag, T>, Dealloc<'tag, T>) {
        let pointer = Pointer {
            _tag: Tag(PhantomData),
            data: self.pointer.data,
        };
        let permission = Write(Token(PhantomData));
        let dealloc = Dealloc(PhantomData);
        std::mem::forget(self);
        (pointer, permission, dealloc)
    }

    pub fn from_raw(
        pointer: Pointer<'tag>,
        permission: Write<'tag, T>,
        _dealloc: Dealloc<'tag, T>,
    ) -> Self {
        Value {
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
            permission: Read(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable);
    }
    pub fn borrow_mut(&self, f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag, T>)) {
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

impl<'tag, T> Drop for Value<'tag, T> {
    fn drop(&mut self) {
        unsafe {
            let layout = Layout::new::<T>();
            dealloc(self.pointer.data, layout);
        }
    }
}

#[derive(Copy, Clone)]
pub struct Ref<'tag, T> {
    pub(crate) pointer: Pointer<'tag>,
    pub(crate) permission: Read<'tag, T>,
}

impl<'tag, T> Ref<'tag, T> {
    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
    }
    pub fn borrow(&self, f: impl for<'retag> FnOnce(Ref<'retag, T>)) {
        let immutable = Ref {
            permission: Read(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: self.pointer.data,
            },
        };
        f(immutable);
    }
    pub fn split(self) -> (Pointer<'tag>, Read<'tag, T>) {
        (self.pointer, self.permission)
    }
}

pub struct RefReserved<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Reserved<'tag, T>,
}

impl<'tag, T> RefReserved<'tag, T> {
    pub fn activate(self, _token: Token<'tag, T>) -> RefMut<'tag, T> {
        RefMut {
            permission: Write(Token(PhantomData)),
            pointer: self.pointer,
        }
    }
    pub fn read(&self, f: impl for<'b> FnOnce(&'b T)) {
        self.pointer.read(&self.permission, f)
    }
    pub fn borrow_mut(
        &self,
        token: Token<'tag, T>,
        f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag, T>),
    ) -> Token<'tag, T> {
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

    pub fn split(self) -> (Pointer<'tag>, Reserved<'tag, T>) {
        (self.pointer, self.permission)
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
        f: impl for<'retag> FnOnce(RefReserved<'retag, T>, Token<'retag, T>),
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

    pub fn split(self) -> (Pointer<'tag>, Write<'tag, T>) {
        (self.pointer, self.permission)
    }
}

pub struct RefMut<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Write<'tag, T>,
}

#[cfg(test)]
mod tests {
    use crate::refs::*;
    #[test]
    fn read_from_ref() {
        let value = Value::new(1);
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
        let value = Value::new(1);
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
        let value = Value::new(1);
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
        let value = Value::new(1);
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
        let value = Value::new(1);
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
}
