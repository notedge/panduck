use std::mem::swap;

use super::*;

pub struct ConfigField<T> {
    // none: not set, use default
    inner: Option<T>,
}

impl<T> Default for ConfigField<T> {
    fn default() -> Self {
        Self { inner: None }
    }
}

impl<T> ConfigField<T> {
    pub fn get(&self) -> Cow<T>
    where
        T: Default + Clone,
    {
        match &self.inner {
            None => Cow::Owned(T::default()),
            Some(v) => Cow::Borrowed(&v),
        }
    }
    pub fn get_mut(&mut self) -> &mut T
    where
        T: Default,
    {
        if let None = &self.inner {
            self.inner = Some(T::default())
        }
        self.inner.as_mut().unwrap()
    }
    pub fn set(&mut self, value: T) -> Option<&mut T>
    where
        T: Default,
    {
        match &mut self.inner {
            Some(s) => {
                let mut new = value;
                swap(&mut new, s);
                return Some(s);
            }
            None => {
                self.inner = Some(value);
                None
            }
        }
    }
}
