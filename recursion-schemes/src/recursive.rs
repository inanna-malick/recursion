use crate::frame::MappableFrame;

pub mod collapse;
pub mod expand;

/// A type with an associated frame type via which instances can be expanded or collapsed
pub trait HasRecursiveFrame {
    type FrameToken: MappableFrame;
}
