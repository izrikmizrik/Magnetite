//! `horizon.random(seed)`: Java's `Random` (the game's `LegacyRandom`), the same numbers on
//! every machine for the same seed, in place of `math.random`.
use horizon_core::random::{LegacyRandom, RandomSource};
use mlua::{UserData, UserDataMethods};

/// A seeded random a script draws from.
pub(crate) struct LuaRandom(LegacyRandom);

impl LuaRandom {
    pub(crate) fn new(seed: i64) -> Self {
        Self(LegacyRandom::new(seed))
    }
}

impl UserData for LuaRandom {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // 0 to bound - 1.
        methods.add_method_mut("next_int", |_, this, bound: i32| {
            this.0
                .next_int_bound(bound)
                .map_err(|_| mlua::Error::runtime("next_int: the bound must be above 0"))
        });
        methods.add_method_mut("next_float", |_, this, ()| Ok(this.0.next_float()));
        methods.add_method_mut("next_double", |_, this, ()| Ok(this.0.next_double()));
        methods.add_method_mut("next_boolean", |_, this, ()| Ok(this.0.next_boolean()));
    }
}
