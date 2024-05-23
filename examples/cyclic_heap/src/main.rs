use ghostborrows::*;

pub struct Child<'p> {
    pub child_data: i32,
    pub parent: Pointer<'p>,
}
pub struct Parent<'p, 'c> {
    pub parent_data: i32,
    pub child: Option<(Pointer<'c>, Write<'c, Child<'p>>, Dealloc<'c, Child<'p>>)>,
}

impl Default for Parent<'_, '_> {
    fn default() -> Self {
        Self {
            parent_data: 0,
            child: None,
        }
    }
}
pub struct HeapWrapper<'p, 'c> {
    pub inner: Value<'p, Parent<'p, 'c>>,
}

impl<'p, 'c> HeapWrapper<'p, 'c> {
    pub fn new() -> Self {
        let inner = Value::new(Parent::default());
        let (ptr, perm, dealloc) = inner.into_raw();
        init(ptr, &perm);
        let inner = Value::from_raw(ptr, perm, dealloc);
        Self { inner }
    }
}

pub fn init<'p, 'c>(parent: Pointer<'p>, perm: &Write<'p, Parent<'p, '_>>)
where
    'p: 'c,
{
    let child_allocation = Value::new(Child {
        child_data: 0,
        parent,
    });
    let (child, child_perm, child_dealloc) = child_allocation.into_raw();

    parent.as_mut(perm).child = Some((child, child_perm, child_dealloc));
}

fn main() {
    let _ = HeapWrapper::new();
}