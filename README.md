# Ghost Borrows 👻
:warning: This is a research experiment—it is unsound, has serious gaps in functionality, and really only makes sense to use as a component of a larger, deductive verification framework.

---
Ghost Borrows is a static API for safely handling reference to raw pointer conversion using *Ghost Permissions*. This verification technique was used to implement the [GhostCell](https://plv.mpi-sws.org/rustbelt/ghostcell/) type, and is foundational to the [Verus](https://github.com/verus-lang/verus) verifier. Here, we use this approach
to manage the *provenance* of references under Rust's [Tree Borrows](https://github.com/Vanille-N/tree-borrows/blob/master/full/main.pdf) aliasing model.

It is sometimes necessary for developers to cast Rust's safe reference types (`&T`, `&mut T`) into raw pointers (`*const T`, `*mut T`). The borrow checker does not restrict raw pointer use, but it still makes aliasing assumptions based on the origin, or *provenance*, of a pointer. If raw pointers are used incorrectly, it can lead to [undefined behavior](https://doc.rust-lang.org/reference/behavior-considered-undefined.html). Currently, most Rust developers who write `unsafe` code use [Miri](https://github.com/rust-lang/miri) to dynamically detect aliasing violations. Verus can statically verify raw pointer use, but it does not yet support reasoning about the provenance of pointers cast from reference types. This library provides one possible form this abstract type could take, supporing static validation of raw pointer use under Tree Borrows.