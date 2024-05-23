use ghostborrows::*;

fn main() {
    let value = Value::new(1);
    value.borrow_mut(|r1, token1| {
        let activated = r1.activate(token1);
        activated.borrow_mut(|_, _| {
            let (_, _) = r1.split();
        });
    });
}