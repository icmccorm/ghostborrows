use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub(crate) struct Tag<'tag>(pub(crate) PhantomData<*mut &'tag ()>);

pub struct Token<'tag>(pub(crate) PhantomData<*mut &'tag ()>);

pub trait AllowsRead<'tag, T> {}
pub trait AllowsWrite<'tag, T> {}

pub struct Reserved<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
impl<'tag, T> AllowsRead<'tag, T> for Reserved<'tag, T> {}

#[derive(Copy, Clone)]
pub struct Frozen<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
impl<'tag, T> AllowsRead<'tag, T> for Frozen<'tag, T> {}

pub struct Active<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
impl<'tag, T> AllowsRead<'tag, T> for Active<'tag, T> {}
impl<'tag, T> AllowsWrite<'tag, T> for Active<'tag, T> {}

pub struct Dealloc<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
