use crate::{
    frame::MappableFrame,
    recursive::{collapse::Collapsable, HasRecursiveFrame},
};

use self::frame::PartiallyApplied;

pub mod fix;
pub mod frame;

pub enum Peano<Next> {
    Succ(Next),
    Zero,
}

impl MappableFrame for Peano<PartiallyApplied> {
    type Frame<Next> = Peano<Next>;

    fn map_frame<A, B>(input: Self::Frame<A>, mut f: impl FnMut(A) -> B) -> Self::Frame<B> {
        match input {
            Peano::Succ(x) => Peano::Succ(f(x)),
            Peano::Zero => Peano::Zero,
        }
    }
}

impl HasRecursiveFrame for usize {
    type FrameToken = Peano<PartiallyApplied>;
}

impl Collapsable for usize {
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        if self == 0 {
            Peano::Zero
        } else {
            Peano::Succ(self - 1)
        }
    }
}

// #[test]
// fn hey_check_this_out() {
//     let x: Vec<usize> = vec![1, 2, 3];

//     let sum_1 = x.collapse_frames(|frame| match frame {
//         ListFrame::Cons(elem, acc) => elem + acc,
//         ListFrame::Nil => 0,
//     });

//     let sum_2 = x.iter().fold(0, |elem, acc| elem + acc);

//     assert_eq!(sum_1, sum_2);
// }

#[test]
fn peano_numbers() {
    let x: usize = 3;

    let peano_repr: String = x.collapse_frames(|frame: Peano<String>| match frame {
        Peano::Succ(mut acc) => {
            acc.push_str(" + 1");
            acc
        }
        Peano::Zero => "0".to_string(),
    });

    assert_eq!("0 + 1 + 1 + 1", &peano_repr);
}
