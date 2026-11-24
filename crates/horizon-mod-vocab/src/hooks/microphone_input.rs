//! The `microphone_input` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// What the microphone recorded since the last client tick, for mod `mod_id`, which started
    /// recording with `start_microphone`: `samples` are mono, from -1 to 1, at `sample_rate`
    /// samples per second. Once every client tick while it records and something was recorded;
    /// only mod `mod_id` hears it. Up to two seconds wait for a tick, older samples are dropped.
    #[hook(
        side = "client",
        contract = "listen",
        owner = "mod_id",
        neoforge = "none (Magnetite's own: Simple Voice Chat records with its own audio code)"
    )]
    fn microphone_input();
}
