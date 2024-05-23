use ghostborrows::*;

fn main() {
    let value = Value::new(1);
    value.borrow(|r1| {
        r1.borrow(|r2| {
            let (ptr1, _) = r1.split();
            let (_, perm2) = r2.split();
            let _ = Ref {
                pointer: ptr1,
                permission: perm2,
            };
        })
    });
}
