use crate::functor::Functor;
use crate::recursive::{Collapse, Expand};
use crate::recursive_tree::arena_eval::ArenaIndex;
use crate::recursive_tree::RecursiveTree;

#[derive(Debug, Clone)]
pub struct NTreeLayer<Val, A> {
    val: Val,
    children: Vec<A>,
}

pub type RecursiveNTree<V> = RecursiveTree<NTreeLayer<V, ArenaIndex>, ArenaIndex>;

impl<A, B, V> Functor<B> for NTreeLayer<V, A> {
    type To = NTreeLayer<V, B>;
    type Unwrapped = A;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, f: F) -> Self::To {
        Self::To {
            val: self.val,
            children: self.children.into_iter().map(f).collect(),
        }
    }
}

pub fn depth<V>(r: RecursiveNTree<V>) -> usize {
    r.collapse_layers(|layer| layer.children.iter().max().map_or(1, |n| n + 1))
}

pub fn max<V: Ord>(r: RecursiveNTree<V>) -> Option<V> {
    r.collapse_layers(|layer| layer.children.into_iter().filter_map(|x| x).max())
}

/// A linked list of characters. Not good or idiomatic, but it provides a nice minimal example
#[derive(Debug, Clone, Copy)]
pub enum CharLinkedList<A> {
    Cons(char, A),
    Nil,
}

impl<A, B> Functor<B> for CharLinkedList<A> {
    type To = CharLinkedList<B>;
    type Unwrapped = A;

    fn fmap<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            CharLinkedList::Cons(c, a) => CharLinkedList::Cons(c, f(a)),
            CharLinkedList::Nil => CharLinkedList::Nil,
        }
    }
}

pub type RecursiveString = RecursiveTree<CharLinkedList<ArenaIndex>, ArenaIndex>;

pub fn from_str(s: &str) -> RecursiveString {
    RecursiveString::expand_layers(s.chars(), |mut it| {
        if let Some(c) = it.next() {
            CharLinkedList::Cons(c, it)
        } else {
            CharLinkedList::Nil
        }
    })
}

pub fn to_str(r: RecursiveString) -> String {
    r.collapse_layers(|cll| match cll {
        CharLinkedList::Cons(c, s) => format!("{}{}", c, s),
        CharLinkedList::Nil => String::new(),
    })
}
