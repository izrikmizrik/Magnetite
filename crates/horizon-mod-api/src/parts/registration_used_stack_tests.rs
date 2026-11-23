#[cfg(test)]
mod tests {
    use crate::testing::FakeHost;
    use crate::{Identifier, ItemStack, UsedStack};

    fn stack(item: &str, count: u32) -> ItemStack {
        ItemStack::new(crate::__private::item_key(item).unwrap(), count).unwrap()
    }

    /// A block's `use_item_on` changes the used stack through the event: untouched, the hand
    /// keeps its stack; a count, a replacement or an emptied hand is what the game writes.
    #[test]
    fn the_used_stack_reports_only_what_a_part_changed() {
        let mut host = FakeHost::new();
        host.registries
            .resolve_items(["minecraft:bowl", "minecraft:bread"].map(|id| Identifier::parse(id).unwrap()))
            .unwrap();
        let skillet = stack("minecraft:bowl", 4);
        let untouched = UsedStack::new(Some(skillet.clone()));
        assert_eq!(untouched.stack(), Some(&skillet));
        assert_eq!(untouched.into_changed(), None);

        let mut counted = UsedStack::new(Some(skillet.clone()));
        counted.set_count(3);
        assert_eq!(counted.stack().map(ItemStack::count), Some(3));
        assert!(counted.is_changed());

        let mut emptied = UsedStack::new(Some(skillet.clone()));
        emptied.set_count(0);
        assert_eq!(emptied.into_changed(), Some(None));

        let flipped = stack("minecraft:bread", 1);
        let mut replaced = UsedStack::new(Some(skillet));
        replaced.replace(Some(flipped.clone()));
        assert_eq!(replaced.into_changed(), Some(Some(flipped)));
    }
}
