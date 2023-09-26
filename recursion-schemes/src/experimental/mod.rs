use crate::{frame::MappableFrame, recursive::collapse::Collapsable};

use self::frame::PartiallyApplied;

pub mod compact;
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

impl Collapsable for usize {
    type FrameToken = Peano<PartiallyApplied>;

    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        if self == 0 {
            Peano::Zero
        } else {
            Peano::Succ(self - 1)
        }
    }
}

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
