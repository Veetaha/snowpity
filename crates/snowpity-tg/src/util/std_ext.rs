use easy_ext::ext;

pub(crate) mod prelude {
    pub(crate) use super::ErrorExt as _;
    pub(crate) use super::IntoIteratorExt as _;
}

#[ext(ErrorExt)]
pub(crate) impl<E> E
where
    E: std::error::Error + ?Sized,
{
    fn display_chain(&self) -> display_error_chain::DisplayErrorChain<&Self> {
        display_error_chain::DisplayErrorChain::new(self)
    }
}

pub(crate) fn type_name_of_val<T: ?Sized>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

#[ext(IntoIteratorExt)]
pub(crate) impl<T: IntoIterator> T {
    fn map_collect<C, R>(self, map: impl FnMut(Self::Item) -> R) -> C
    where
        C: FromIterator<R>,
        Self: Sized,
    {
        self.into_iter().map(map).collect()
    }
}
