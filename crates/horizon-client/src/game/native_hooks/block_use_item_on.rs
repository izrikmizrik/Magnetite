use super::*;

/// The hook points. Each does nothing in Horizon.
impl Game<'_> {
    /// `BlockBehaviour.useItemOn`: the block's own behaviour, with the
    /// held item, on the server for each player. An answer replaces the
    /// template's behaviour; `TryWithEmptyHand` tries `useWithoutItem` on
    /// the main hand, and `Pass` goes on to the item's `useOn`. `used` is the stack the click
    /// was made with: change it (`replace`, `set_count`, `stack_mut`) and the hand that clicked
    /// gets it once the call returns.
    pub(in crate::game) fn native_use_item_on(
        &mut self,
        state: BlockStateId,
        click: &BlockClick<'_>,
        used: &mut mod_api::UsedStack,
        answer: &mut Option<mod_api::InteractionResult>,
    ) {
        self.fire_use_item_on(state, click, used, answer);
    }
}
