use std::mem::swap;

use super::*;

pub struct ConfigField<T> {
    // None: not set, use default
    // Some:
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
            Some(v) => Cow::Borrowed(v),
        }
    }
    pub fn get_mut(&mut self) -> &mut T
    where
        T: Default,
    {
        if self.inner.is_none() {
            self.inner = Some(T::default())
        }
        self.inner.as_mut().unwrap()
    }
    pub fn set(&mut self, value: T) -> Option<T>
    where
        T: Default,
    {
        let have_value = self.inner.is_some();
        match have_value {
            true => {
                let mut new = Some(value);
                swap(&mut new, &mut self.inner);
                new
            }
            false => {
                self.inner = Some(value);
                None
            }
        }
    }
    pub fn merge(&mut self, rhs: ConfigField<T>) {
        match (&mut self.inner, rhs.inner) {
            // left side config have have been override by right
            (Some(lhs), Some(rhs)) => *lhs = rhs,
            // nothing changed because rhs not set
            (Some(_), None) => {}
            // use rhs config
            (None, Some(rhs)) => self.inner = Some(rhs),
            // skip
            (None, None) => {}
        }
    }
}
