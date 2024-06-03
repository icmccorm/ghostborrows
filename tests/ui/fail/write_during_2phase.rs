use ghostborrows::*;

fn main() {
    /*
     
     // Not UB according to Stacked Borrows.
     // Should be UB according to Tree Borrows.

     fn write_during_2phase() {
        let x = &mut 0u8;
        let xraw = x as *mut u8;
        print(
            x,
            unsafe { *xraw += 1 };
        )
     }
    
    */

    let value = Value::new(0);
    value.borrow_mut(|x, token| {
        let (pointer, perm) = x.split();
        x.borrow_mut(token, |_, _| {
            *pointer.as_mut(perm) += 1;
        });
    });
}