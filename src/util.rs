pub trait OnlyIterable: Iterator where Self: Sized {
    fn only(mut self) -> Option<Self::Item> {
        let result = self.next();
        if let Some(_) = self.next() {
            panic!("Unexpected second value in OnlyIterator!");
        }
        result
    }
}

impl<I> OnlyIterable for I where I: Iterator {}

use std::hash::Hash;
use std::collections::HashSet;
pub struct UniqueIterator<T, I> where T: Hash+Eq+Clone, I: Iterator<Item=T> {
    source: I,
    previous: HashSet<T>
}
impl<T, I> Iterator for UniqueIterator<T, I> where T: Hash+Eq+Clone, I: Iterator<Item=T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(value) = self.source.next() {
            if self.previous.insert(value.clone()) {
                return Some(value);
            }
        }

        return None;
    }
}

pub trait UniqueIterable<T> where T: Hash+Eq+Clone, Self: Iterator<Item=T> + Sized {
    fn unique(self) -> UniqueIterator<T, Self> {
        UniqueIterator { source: self, previous: HashSet::new() }
    }
}

impl<T, I> UniqueIterable<T> for I where T: Hash+Eq+Clone, I: Iterator<Item=T> {}
