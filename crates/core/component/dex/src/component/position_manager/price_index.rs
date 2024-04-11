use cnidarium::StateWrite;

use crate::{
    lp::position::{self, Position},
    state_key::engine,
    DirectedTradingPair,
};

pub(crate) trait PositionByPriceIndex: StateWrite {
    fn index_position_by_price(&mut self, position: &position::Position, id: &position::Id) {
        let (pair, phi) = (position.phi.pair, &position.phi);
        if position.reserves.r2 != 0u64.into() {
            // Index this position for trades FROM asset 1 TO asset 2, since the position has asset 2 to give out.
            let pair12 = DirectedTradingPair {
                start: pair.asset_1(),
                end: pair.asset_2(),
            };
            let phi12 = phi.component.clone();
            self.nonverifiable_put_raw(engine::price_index::key(&pair12, &phi12, &id), vec![]);
            tracing::debug!("indexing position for 1=>2 trades");
        }

        if position.reserves.r1 != 0u64.into() {
            // Index this position for trades FROM asset 2 TO asset 1, since the position has asset 1 to give out.
            let pair21 = DirectedTradingPair {
                start: pair.asset_2(),
                end: pair.asset_1(),
            };
            let phi21 = phi.component.flip();
            self.nonverifiable_put_raw(engine::price_index::key(&pair21, &phi21, &id), vec![]);
            tracing::debug!("indexing position for 2=>1 trades");
        }
    }

    fn deindex_position_by_price(&mut self, position: &Position, id: &position::Id) {
        tracing::debug!("deindexing position");
        let pair12 = DirectedTradingPair {
            start: position.phi.pair.asset_1(),
            end: position.phi.pair.asset_2(),
        };
        let phi12 = position.phi.component.clone();
        let pair21 = DirectedTradingPair {
            start: position.phi.pair.asset_2(),
            end: position.phi.pair.asset_1(),
        };
        let phi21 = position.phi.component.flip();
        self.nonverifiable_delete(engine::price_index::key(&pair12, &phi12, &id));
        self.nonverifiable_delete(engine::price_index::key(&pair21, &phi21, &id));
    }
}
impl<T: StateWrite + ?Sized> PositionByPriceIndex for T {}
