use std::marker::PhantomData;

#[derive(Copy, Clone)]
pub(crate) struct Tag<'tag>(pub(crate) PhantomData<*mut &'tag ()>);

pub struct Token<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);

pub trait AllowsRead<'tag, T> {}
pub trait AllowsWrite<'tag, T> {}

pub struct Reserved<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);

impl<'tag, T> AllowsRead<'tag, T> for Reserved<'tag, T> {}

#[derive(Copy, Clone)]
pub struct Read<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
impl<'tag, T> AllowsRead<'tag, T> for Read<'tag, T> {}

pub struct Write<'tag, T>(pub(crate) Token<'tag, T>);
impl<'tag, T> AllowsRead<'tag, T> for Write<'tag, T> {}
impl<'tag, T> AllowsWrite<'tag, T> for Write<'tag, T> {}

pub struct Dealloc<'tag, T>(pub(crate) PhantomData<*mut &'tag T>);
