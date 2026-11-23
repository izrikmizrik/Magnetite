//! The hooks Lua sees come from the same catalog the native and wasm surfaces are generated from
//! (`crates/horizon-mod-vocab/src/hooks`): this writes that list for the runtime to check every
//! name a script uses against.
use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let declarations = root.join(horizon_mod_api_gen::DECLARATIONS);
    println!("cargo:rerun-if-changed={}", declarations.display());
    let points = horizon_mod_api_gen::hook_points(&root).expect("the hook catalog loads");
    let mut out = String::from(
        "/// Every hook of the catalog: its name, contract version and whether it is a Part's.\n\
         pub(crate) static CATALOG: &[CatalogHook] = &[\n",
    );
    for point in points {
        writeln!(
            out,
            "    CatalogHook {{ name: {:?}, version: {}, scoped: {} }},",
            point.name, point.version, point.scoped
        )
        .unwrap();
    }
    out.push_str("];\n");
    let path = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("lua_catalog.rs");
    std::fs::write(path, out).unwrap();
}
