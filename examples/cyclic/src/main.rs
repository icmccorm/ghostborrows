use ghostborrows::*;
use std::cell::RefCell;

pub struct Child<'p> {
    pub child_data: i32,
    pub parent: Pointer<'p>,
}

#[derive(Default)]
pub struct Parent<'p, 'c> {
    pub parent_data: i32,
    pub child: Option<(Pointer<'c>, Write<'c, Child<'p>>, Dealloc<'c, Child<'p>>)>,
}

pub fn init<'p>(parent: Pointer<'p>, perm: &Write<'p, Parent<'p, '_>>) {
    let child_allocation = Value::new(Child {
        child_data: 0,
        parent,
    });

    let (child, child_perm, child_dealloc) = child_allocation.into_raw();

    parent.as_mut(perm).child = Some((child, child_perm, child_dealloc));
}

pub fn deinit<'p>(parent: Pointer<'p>, perm: &Write<'p, Parent<'p, '_>>) {
    let (child, child_perm, child_dealloc) = parent.as_mut(perm).child.take().unwrap();
    drop(Value::from_raw(child, child_perm, child_dealloc));
}

#[derive(Default)]
pub struct HeapWrapper<'p, 'c> {
    pub inner: Option<Value<'p, Parent<'p, 'c>>>,
}

impl<'p, 'c> HeapWrapper<'p, 'c> {
    pub fn new() -> Self {
        let inner = Value::new(Parent::default());
        let (ptr, perm, dealloc) = inner.into_raw();
        init(ptr, &perm);
        let inner = Some(Value::from_raw(ptr, perm, dealloc));
        Self { inner }
    }
}

impl<'p, 'c> Drop for HeapWrapper<'p, 'c> {
    fn drop(&mut self) {
        let (pointer, permission, dealloc) = self.inner.take().unwrap().into_raw();
        deinit(pointer, &permission);
        drop(Value::from_raw(pointer, permission, dealloc));
    }
}

pub struct StackWrapper<'p, 'c> {
    pub inner: RefCell<(Pointer<'p>, Write<'p, Parent<'p, 'c>>)>,
}

impl<'p, 'c> StackWrapper<'p, 'c> {
    pub fn new(value: &mut Parent<'p, 'c>) -> Self {
        let (pointer, permission) = split_mut!(value);
        init(pointer, &permission);
        Self {
            inner: RefCell::new((pointer, permission)),
        }
    }
}

impl<'p, 'c> Drop for StackWrapper<'p, 'c> {
    fn drop(&mut self) {
        let (pointer, permission) = self.inner.get_mut();
        deinit(*pointer, permission);
    }
}

fn main() {
    let _ = HeapWrapper::new();
    let mut p = Parent {
        parent_data: 0,
        child: None,
    };
    let _ = StackWrapper::new(&mut p);
}
