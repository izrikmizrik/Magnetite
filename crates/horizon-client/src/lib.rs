//! Gameplay and startup, above the reusable client library.
// No `#![recursion_limit]` or other inner attribute horizon-mixins doesn't know here: the
// launcher's horizon-mixins then rejects every native mod targeting this crate (#8169).
#![forbid(unsafe_code)]
extern crate self as horizon_client;
pub use horizon_client_lib::carrier_store;
pub use horizon_client_lib::*;
#[doc(hidden)]
pub mod entry;

// The game's allocator lives in the library, not the executable: the prebuilt game (arismoko/horizon#589) is this crate as a
// dylib, and the allocator must be the one its code and the mods' executable share. The Windows and macOS archives
// build with `system-allocator`: their loaders bind each library to std's own allocator, which everything then
// shares (arismoko/horizon#2394, arismoko/horizon#3877).
#[cfg(not(feature = "system-allocator"))]
#[global_allocator]
static ALLOCATOR: horizon_threads::GameAllocator = horizon_threads::GAME_ALLOCATOR;
pub mod game;
/// The boot loop a phone (or a PC without a launcher) runs before the game: sign-in, the game
/// files, then the hand-over (`docs/unification/mobile-boot.md`). It lives in the game area
/// (`game::boot`) because it draws with the game's own loading renderer; this is its public path.
pub use game::boot;
pub mod orb_limits;
