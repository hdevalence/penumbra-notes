use super::*;

/// A builder for a top of a frontier.
pub struct Builder<Item: Built + Focus>(<Nested<Item> as Built>::Builder)
where
    Item::Complete: Built;

impl<Item: Focus + Built> Built for Top<Item>
where
    Item::Complete: Built,
{
    type Builder = Builder<Item>;

    fn build(global_position: u64, index: u64) -> Self::Builder {
        Builder(Nested::build(global_position, index))
    }
}

impl<Item: Built + Focus> Build for Builder<Item>
where
    Item::Complete: Built,
{
    type Output = Top<Item>;

    fn go(self, instruction: Instruction) -> Result<IResult<Self>, HitBottom<Self>> {
        use IResult::*;

        self.0
            .go(instruction)
            .map(|r| match r {
                Complete(inner) => Complete(Top {
                    inner: Some(inner),
                    track_forgotten: TrackForgotten::Yes,
                }),
                Incomplete(builder) => Incomplete(Builder(builder)),
            })
            .map_err(|HitBottom(builder)| HitBottom(Builder(builder)))
    }

    fn is_started(&self) -> bool {
        self.0.is_started()
    }

    fn index(&self) -> u64 {
        self.0.index()
    }

    fn height(&self) -> u8 {
        self.0.height()
    }

    fn min_required(&self) -> usize {
        self.0.min_required()
    }
}
