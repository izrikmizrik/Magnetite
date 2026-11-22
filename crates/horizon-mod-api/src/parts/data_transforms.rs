//! Validate and keep load-time data transform registrations.
use super::*;
impl DataTransformsRegistrar for PartRegistrar {
    fn add_data_transform(&mut self, kind: &str, transform: Arc<dyn crate::DataTransform>) {
        if ![crate::DataFiles::BIOME_MODIFIER, crate::DataFiles::RECIPE].contains(&kind) {
            self.refused.push(format!(
                "{} registered a data transform of unknown kind {kind:?}",
                self.mod_id
            ));
            return;
        }
        self.parts.data_transforms.push((
            self.mod_index,
            self.mod_id,
            kind.to_owned(),
            transform,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Upper;
    impl crate::DataTransform for Upper {
        fn transform(&self, mut data: crate::DataFiles) -> Result<crate::DataFiles, String> {
            for (_, text) in &mut data.files {
                *text = text.to_uppercase();
            }
            data.files.push(("ruby:added".into(), "{}".into()));
            Ok(data)
        }
    }
    struct Broken;
    impl crate::DataTransform for Broken {
        fn transform(&self, _: crate::DataFiles) -> Result<crate::DataFiles, String> {
            panic!("a broken transform")
        }
    }
    struct Mod;
    #[mod_api::__subscriptions]
    impl Hooks for Mod {
        fn register(&self, r: &mut dyn Registrar, _: &RegistrationContext) {
            r.register_data_transform(crate::DataFiles::RECIPE, Broken);
            r.register_data_transform(crate::DataFiles::RECIPE, Upper);
        }
    }

    /// Registered transforms run in order over the kind's files; one that panics is skipped and
    /// the files come back in id order.
    #[test]
    fn transforms_run_in_order_and_a_broken_one_changes_nothing() {
        crate::parts::tests::install_parts_fixture(&[ModEntry::new("ruby", Box::new(Mod))])
            .unwrap();
        let out = crate::__private::run_data_transforms(crate::DataFiles::new(
            crate::DataFiles::RECIPE,
            vec![("minecraft:z".into(), "{\"a\":1}".into())],
        ));
        assert_eq!(
            out.files,
            [
                ("minecraft:z".to_owned(), "{\"A\":1}".to_owned()),
                ("ruby:added".to_owned(), "{}".to_owned()),
            ]
        );
        let other = crate::__private::run_data_transforms(crate::DataFiles::new(
            crate::DataFiles::BIOME_MODIFIER,
            vec![("minecraft:z".into(), "{}".into())],
        ));
        assert_eq!(other.files, [("minecraft:z".to_owned(), "{}".to_owned())]);
    }

    #[test]
    fn an_unknown_kind_refuses_the_mod() {
        let mut registrar = super::PartRegistrar::new("ruby");
        registrar.add_data_transform("loot", Arc::new(Upper));
        assert_eq!(
            registrar.refused,
            ["ruby registered a data transform of unknown kind \"loot\""]
        );
    }
}
