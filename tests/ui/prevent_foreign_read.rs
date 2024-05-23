use ghost_borrows::*;

fn main() {
    OwnedValue::new(1).borrow_mut(|r1, token1| {
        let activated = r1.activate(token1);
        activated.borrow_mut(|_, _| {
            activated.read(|r1m| print!("{}", *r1m) );
        });
    });
}