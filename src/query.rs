use std::marker::PhantomData;

pub struct Query<T> {
    _phtm: PhantomData<T>,
}

impl<T> Query<T> {
    pub(crate) fn new() -> Self {
        Self { _phtm: PhantomData }
    }
    pub fn iter(self) -> QueryIter<T> {
        QueryIter { _phtm: PhantomData }
    }
}

pub struct QueryIter<T> {
    _phtm: PhantomData<T>,
}

impl<T> Iterator for QueryIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
