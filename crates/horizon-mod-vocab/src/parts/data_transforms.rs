//! Load-time transforms of the data mods' files are read from: a mod reads and rewrites one kind
//! of JSON before the game parses it.
use super::*;
use crate::wire::{Reader, Wire, WireError, Writer};

/// One kind of data a load transform reads and changes, and what the game knows at that moment.
///
/// `files` are the kind's files by id (`mymod:thing`; a vanilla file is `minecraft:`'s), each
/// its JSON text, in id order. Add, remove or rewrite entries: the game parses what comes back,
/// again in id order. The biomes and biome tags are read-only context, after the world's data
/// packs, for a transform that sorts by biome (a modifier filtering by temperature).
#[derive(Clone, Debug, Default, PartialEq, horizon_mod_api_macros::Wire)]
#[non_exhaustive]
pub struct DataFiles {
    /// The kind: [`DataFiles::BIOME_MODIFIER`] or [`DataFiles::RECIPE`].
    pub kind: String,
    /// The files, id and JSON text, in id order.
    pub files: Vec<(String, String)>,
    /// The world's biomes, id and JSON text, in id order: filled for biome modifiers.
    pub biomes: Vec<(String, String)>,
    /// The biome tags, id (without `#`) and the biomes it holds, nested tags resolved, in id
    /// order: filled for biome modifiers.
    pub biome_tags: Vec<(String, Vec<String>)>,
}

impl DataFiles {
    /// NeoForge's biome modifiers (`data/<ns>/neoforge/biome_modifier/`), mods' own types
    /// included. Asked when a world opens, with its packs' biomes and tags in place; a
    /// modifier of a type the game doesn't read can be rewritten into ones it does
    /// (`neoforge:add_features` with an explicit biome list).
    pub const BIOME_MODIFIER: &'static str = "biome_modifier";
    /// Recipes (`data/<ns>/recipe/`), the world's packs over the game's own. Asked at every
    /// load and `/reload`, on a server and on a client that loads its own; a recipe with an
    /// `OR` of ingredients a type can't read can be split into ones it can.
    pub const RECIPE: &'static str = "recipe";

    /// A set of `kind`'s files, with no context.
    pub fn new(kind: &str, files: Vec<(String, String)>) -> Self {
        Self {
            kind: kind.to_owned(),
            files,
            ..Self::default()
        }
    }
}

/// A mod's load-time transform of one kind of data (`Registrar::register_data_transform`).
///
/// It runs on the loading thread, once per load of its kind, mods in load order, each seeing
/// the files the one before it left. An error or a panic is written to the log with the mod's
/// id and the files stay as they were: a broken transform never stops the load. Keep it a pure
/// function of what it is given (no clock, no random) so every machine loads the same world.
pub trait DataTransform: Send + Sync {
    /// Read and change `data`, giving it back.
    fn transform(&self, data: DataFiles) -> Result<DataFiles, String>;
}
crate::wire::wire_handlers::handler!(dyn DataTransform, "DataTransform");

/// Registrar operations for load-time data transforms.
pub trait DataTransformsRegistrar {
    /// Register `transform` for data of `kind` ([`DataFiles::BIOME_MODIFIER`],
    /// [`DataFiles::RECIPE`]). An unknown kind refuses the mod, naming it.
    fn add_data_transform(&mut self, kind: &str, transform: Arc<dyn DataTransform>);
}

impl dyn Registrar + '_ {
    /// Register a load-time transform of `kind`'s files (see [`DataTransform`]).
    pub fn register_data_transform(&mut self, kind: &str, transform: impl DataTransform + 'static) {
        self.add_data_transform(kind, Arc::new(transform));
    }
}

/// The enabled mods' transforms of `kind`, with their mod ids, in load order.
pub(crate) fn installed_data_transforms(kind: &str) -> Vec<(&'static str, Arc<dyn DataTransform>)> {
    installed().map_or_else(Vec::new, |installed| {
        installed
            .keys
            .data_transforms
            .iter()
            .filter(|(index, _, of, _)| of == kind && crate::mod_index_on(*index))
            .map(|(_, mod_id, _, transform)| (*mod_id, transform.clone()))
            .collect()
    })
}

/// Whether an enabled mod transforms `kind`'s files.
pub fn has_data_transforms(kind: &str) -> bool {
    !installed_data_transforms(kind).is_empty()
}

/// Run the enabled mods' transforms of `data.kind` over `data`, in load order. A transform that
/// fails or panics is logged with its mod's id and changes nothing; the files come back in id
/// order, without repeats (the last entry of an id wins).
pub fn run_data_transforms(mut data: DataFiles) -> DataFiles {
    for (mod_id, transform) in installed_data_transforms(&data.kind) {
        let input = data.clone();
        let kind = data.kind.clone();
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| transform.transform(input))) {
            Ok(Ok(mut out)) => {
                out.kind = kind;
                out.biomes = std::mem::take(&mut data.biomes);
                out.biome_tags = std::mem::take(&mut data.biome_tags);
                data = out;
            }
            Ok(Err(error)) => log::error!("mod {mod_id}: {kind} transform failed: {error}"),
            Err(_) => log::error!("mod {mod_id}: {kind} transform panicked"),
        }
    }
    let files: std::collections::BTreeMap<String, String> = data.files.drain(..).collect();
    data.files = files.into_iter().collect();
    data
}
