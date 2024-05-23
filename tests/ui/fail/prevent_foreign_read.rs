use ghostborrows::*;

fn main() {
    Value::new(1).borrow_mut(|r1, token1| {
        let activated = r1.activate(token1);
        activated.borrow_mut(|_, _| {
            print!("{}", *r1);
        });
    });
}