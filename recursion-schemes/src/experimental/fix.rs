use crate::{
    frame::MappableFrame,
    recursive::{collapse::Collapsable, expand::Expandable},
};

/// heap allocated fix point of some Functor
#[derive(Debug)]
pub struct Fix<F: MappableFrame>(pub Box<F::Frame<Fix<F>>>);

impl<F: MappableFrame> Collapsable for Fix<F> {
    type FrameToken = F;
    fn into_frame(self) -> <Self::FrameToken as MappableFrame>::Frame<Self> {
        *self.0
    }
}

impl<F: MappableFrame> Expandable for Fix<F> {
    type FrameToken = F;
    fn from_frame(val: <Self::FrameToken as MappableFrame>::Frame<Self>) -> Self {
        Fix::new(val)
    }
}

impl<F: MappableFrame> Fix<F> {
    pub fn new(x: F::Frame<Self>) -> Self {
        Self(Box::new(x))
    }
}
