use crate::{
    map_layer::{CoProject, MapLayer, Project},
    Expand,
};

use super::expand_and_collapse;

impl<
        // F, a type parameter of kind * -> * that cannot be represented in rust
        Seed: Project<To = Expandable>,
        Out: CoProject<From = Collapsable>,
        Expandable: MapLayer<Unwrapped = Seed, Layer<()> = U>, // F<Seed>
        Collapsable,
        U: MapLayer<Layer<Out> = Collapsable, Unwrapped = ()>, // F<()>
    > Expand<Seed, Expandable> for Out
{
    fn expand_layers<F: Fn(Seed) -> Expandable>(seed: Seed, expand_layer: F) -> Self {
        expand_and_collapse(seed, expand_layer, CoProject::coproject)
    }
}