use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub(crate) struct Tag<'tag, V>(pub(crate) PhantomData<*mut &'tag V>);

pub struct Token<'tag>(pub(crate) PhantomData<*mut &'tag ()>);

pub trait AllowsRead<'tag> {}
pub trait AllowsWrite<'tag> {}

pub struct Reserved<'tag>(pub(crate) PhantomData<*mut &'tag ()>);
impl<'tag> AllowsRead<'tag> for Reserved<'tag> {}

#[derive(Copy, Clone)]
pub struct Frozen<'tag>(pub(crate) PhantomData<*mut &'tag ()>);
impl<'tag> AllowsRead<'tag> for Frozen<'tag> {}

pub struct Active<'tag>(pub(crate) PhantomData<*mut &'tag ()>);
impl<'tag> AllowsRead<'tag> for Active<'tag> {}
impl<'tag> AllowsWrite<'tag> for Active<'tag> {}

