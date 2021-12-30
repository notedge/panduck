use std::{
    borrow::{Borrow, Cow},
    collections::HashMap,
    lazy::SyncLazy,
    ops::Deref,
};

mod field;

pub struct ConfigList<T> {
    inner: Vec<T>,
}

pub struct ConfigMap<K, V> {
    inner: HashMap<K, V>,
}

// #[derive(Clone, Default)]
// pub struct TestConfig {
//     inner: String,
// }
//
// pub struct Test {
//     ///
//     ///
//     field: ConfigField<TestConfig>,
// }
//
// #[allow(non_upper_case_globals)]
// static TestConfigDefault: SyncLazy<TestConfig> =
//     SyncLazy::new(TestConfig::default);
//
// impl Test {
//     pub fn get_field(&self) -> &TestConfig {
//         match &self.field {
//             ConfigState::None | ConfigState::Default => {
//                 TestConfigDefault.deref()
//             }
//             ConfigState::Default => {}
//             ConfigState::Overridable(_) => {}
//         }
//     }
//     pub fn get_field_mut(&mut self) -> &mut TestConfig {
//         if self.field.is_none() {
//             self.field = Some(TestConfig::default())
//         }
//         self.field.as_mut().unwrap()
//     }
//     pub fn set_field(&mut self, field: TestConfig) -> &mut Self {
//         self.field = Some(field);
//         self
//     }
// }
