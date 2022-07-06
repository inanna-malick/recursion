use crate::recursive_abstract::{Functor, RecursiveStruct, CoRecursive, Recursive};

#[derive(Debug, Clone, Copy)]
pub enum CharLinkedList<A> {
    Cons(char, A),
    Nil,
}

impl<A, B> Functor<B> for CharLinkedList<A> {
    type To = CharLinkedList<B>;
    type Unwrapped = A;

    fn fmap_into<F: FnMut(Self::Unwrapped) -> B>(self, mut f: F) -> Self::To {
        match self {
            CharLinkedList::Cons(c, a) => CharLinkedList::Cons(c, f(a)),
            CharLinkedList::Nil => CharLinkedList::Nil,
        }
    }
}

pub type RecursiveString = RecursiveStruct<CharLinkedList<usize>>;

pub fn from_str(s: &str) -> RecursiveString {
    RecursiveString::ana(s.chars(), |mut it| {
        if let Some(c) = it.next() {
            CharLinkedList::Cons(c, it)
        } else {
            CharLinkedList::Nil
        }
    })
}

pub fn to_str(r: RecursiveString) -> String {
    r.cata( |cll| {
        match cll {
            CharLinkedList::Cons(c, s) => format!("{}{}", c, s),
            CharLinkedList::Nil => String::new(),
        }
    })
}
