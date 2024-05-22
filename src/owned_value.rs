struct OwnedValue<'tag, T: Value> {
    pointer: Pointer<'tag, T>,
    permission: Active<'tag>,
}