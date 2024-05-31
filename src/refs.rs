use crate::perms::{AllowsRead, AllowsWrite, Dealloc, Read, Reserved, Tag, Token, Write};
use std::alloc::{alloc, dealloc, Layout};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
pub struct Pointer<'tag> {
    pub(crate) _tag: Tag<'tag>,
    data: *mut u8,
}
impl<'tag> Pointer<'tag> {
    #[must_use]
    pub fn as_ref<T>(&self, _: &dyn AllowsRead<'tag, T>) -> &T {
        unsafe { &*(self.data as *const T) }
    }

    #[allow(clippy::mut_from_ref)]
    #[must_use]
    pub fn as_mut<T>(&self, _: &mut dyn AllowsWrite<'tag, T>) -> &mut T {
        unsafe { &mut *(self.data.cast::<T>()) }
    }
}

pub struct Value<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Write<'tag, T>,
    _dealloc: Dealloc<'tag, T>,
}

impl<'tag, T> Deref for Value<'tag, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.pointer.as_ref(&self.permission)
    }
}

impl<'tag, T> DerefMut for Value<'tag, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.pointer.as_mut(&mut self.permission)
    }
}

impl<'tag, T> Value<'tag, T> {
    pub fn new(value: T) -> Self {
        let layout = Layout::new::<T>();
        unsafe {
            let data = alloc(layout);
            std::ptr::write(data.cast::<T>(), value);
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
    #[must_use]
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

    #[must_use]
    pub fn from_raw(
        pointer: Pointer<'tag>,
        permission: Write<'tag, T>,
        dealloc: Dealloc<'tag, T>,
    ) -> Self {
        Value {
            pointer,
            permission,
            _dealloc: dealloc,
        }
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

impl<'tag, T> Deref for Ref<'tag, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.pointer.as_ref(&self.permission)
    }
}
impl<'tag, T> From<&T> for Ref<'tag, T> {
    fn from(t: &T) -> Self {
        Ref {
            permission: Read(PhantomData),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: std::ptr::from_ref::<T>(t) as *mut u8,
            },
        }
    }
}

impl<'tag, T> Ref<'tag, T> {
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
    #[must_use]
    pub fn split(self) -> (Pointer<'tag>, Read<'tag, T>) {
        (self.pointer, self.permission)
    }
}

pub struct RefReserved<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Reserved<'tag, T>,
}

impl<'tag, T> Deref for RefReserved<'tag, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.pointer.as_ref(&self.permission)
    }
}

impl<'tag, T> RefReserved<'tag, T> {
    #[must_use]
    #[allow(clippy::needless_pass_by_value)]
    pub fn activate(self, _token: Token<'tag, T>) -> RefMut<'tag, T> {
        RefMut {
            permission: Write(Token(PhantomData)),
            pointer: self.pointer,
        }
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

    #[must_use]
    pub fn pointer(&self) -> Pointer<'tag> {
        self.pointer
    }

    #[must_use]
    pub fn split(&self) -> (Pointer<'tag>, Reserved<'tag, T>) {
        (self.pointer, Reserved(PhantomData))
    }
}

pub struct RefMut<'tag, T> {
    pointer: Pointer<'tag>,
    permission: Write<'tag, T>,
}

impl<'tag, T> Deref for RefMut<'tag, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.pointer.as_ref(&self.permission)
    }
}

impl<'tag, T> DerefMut for RefMut<'tag, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.pointer.as_mut(&mut self.permission)
    }
}

impl<'tag, T> From<&mut T> for RefMut<'tag, T> {
    fn from(t: &mut T) -> Self {
        RefMut {
            permission: Write(Token(PhantomData)),
            pointer: Pointer {
                _tag: Tag(PhantomData),
                data: std::ptr::from_mut::<T>(t).cast::<u8>(),
            },
        }
    }
}

impl<'tag, T> RefMut<'tag, T> {
    #[must_use]
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

    #[must_use]
    pub fn split(self) -> (Pointer<'tag>, Write<'tag, T>) {
        (self.pointer, self.permission)
    }
}

#[cfg(test)]
mod tests {
    use crate::refs::*;
    #[test]
    fn read_from_ref() {
        let value = Value::new(1);
        value.borrow(|r| {
            assert!(*r == 1);
            assert!(*r == *value);
        });
    }

    #[test]
    fn write_from_ref() {
        let value = Value::new(1);
        value.borrow_mut(|ptr, token| {
            assert!(*ptr == 1);
            let mut ptr_mut = ptr.activate(token);
            *ptr_mut = 3;
            assert!(*ptr_mut == 3);
        });
    }

    #[test]
    fn can_create_and_use_multiple_refs() {
        let value = Value::new(1);
        value.borrow(|r1| {
            value.borrow(|r2| {
                value.borrow(|r3| {
                    assert!(*r1 == *r2);
                    assert!(*r2 == *r3);
                    assert!(*r3 == 1);
                });
            });
        });
    }

    #[test]
    fn immutable_reborrow() {
        let value = Value::new(1);
        value.borrow(|r1| {
            r1.borrow(|r2| {
                assert!(*r1 == *r2);
            });
        });
    }

    #[test]
    fn mutable_reborrow() {
        let value = Value::new(1);
        value.borrow_mut(|r1, token1| {
            r1.borrow_mut(token1, |r2, token2| {
                /* We allow foreign reads */
                assert!(*r1 == *r2);
                let mut r2_mut = r2.activate(token2);
                *r2_mut = 2;
                assert!(*r2_mut == 2);
            });
        });
    }

    #[test]
    fn unused_borrow() {
        let value = Value::new(0);
        value.borrow_mut(|x, token| {
            let mut_x = x.activate(token);
            let (pointer, mut perm) = mut_x.split();
            let y = pointer.as_ref(&perm);
            let _z = pointer.as_mut(&mut perm);
            let _val = *y;
        });
    }
}
