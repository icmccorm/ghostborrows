use ghostborrows::*;

fn main() {
    let value = Value::new(0);
    value.borrow_mut(|x, token| {
        let pair = x.split();
        let y = RefReserved::from(pair);
        let z = RefReserved::from(pair);
        *y.activate(token) += 1;
        *z.activate(token) += 1;
    });
}
