//! `--benchmark` left out of the build (the `benchmark` feature off, as in the mobile build):
//! nothing drives a measured run and horizon-bench's report format is not compiled. The two
//! names the command line calls stay, refusing, so it says the mode is missing instead of
//! measuring nothing; benchmark.rs is the real one.
/// What `--benchmark` runs: this build has no scenarios.
pub enum Scenario {}

impl Scenario {
    /// `standard`, a scene, `travel` or `ab:SWITCH` in a build with the feature.
    pub fn parse(_name: &str) -> Result<Scenario, String> {
        Err(
            "this build has no benchmark mode (the `benchmark` feature); \
             build the game with it to measure"
                .to_owned(),
        )
    }
}

/// `--benchmark-set SWITCH=VALUE`: there are no switches without a benchmark to set them for.
pub fn parse_set(_value: &str) -> Result<(String, String), String> {
    Err(
        "this build has no benchmark mode (the `benchmark` feature), so nothing takes \
         --benchmark-set"
            .to_owned(),
    )
}
