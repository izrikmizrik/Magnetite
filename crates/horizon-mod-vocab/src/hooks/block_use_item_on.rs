//! The `block_use_item_on` hooks and helpers: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// `BlockBehaviour.useItemOn`: the block's own behaviour, with the
    /// held item, on the server for each player. An answer replaces the
    /// template's behaviour; `TryWithEmptyHand` tries `useWithoutItem` on
    /// the main hand, and `Pass` goes on to the item's `useOn`. `used` is the stack the click
    /// was made with: change it (`replace`, `set_count`, `stack_mut`) and the hand that clicked
    /// gets it once the call returns, whatever the answer.
    #[hook(
        side = "server",
        contract = "replace",
        out = "answer",
        scope = "block",
        key = "state",
        version = 2,
        neoforge = "none (vanilla BlockBehaviour.useItemOn; PlayerInteractEvent.RightClickBlock cancels or answers before it)"
    )]
    fn use_item_on();
}
