use ghostborrows::*;

type State<'v> = (Value<'v, i32>, Option<(Pointer<'v>, Reserved<'v, i32>)>);

fn main() {
    let s = Value::<State<'_, >>::new((Value::new(0), None));
    s.borrow_mut(|x, token_x| {
        let mut x = x.activate(token_x);
        x.0.borrow_mut(|r, _| {
            x.1 = Some(r.split());
        });
    });
}
