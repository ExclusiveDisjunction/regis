use super::error::PoisonError;

use std::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::fmt::{Debug, Display};

pub struct ReadGuard<'a, T> {
    inner: Result<RwLockReadGuard<'a, T>, PoisonError>
}
impl<'a, T> From<RwLockReadGuard<'a, T>> for ReadGuard<'a, T> {
    fn from(value: RwLockReadGuard<'a, T>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl<'a, T> From<PoisonError> for ReadGuard<'a, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }
    }
} 
impl<'a, T> From<Result<RwLockReadGuard<'a, T>, PoisonError>> for ReadGuard<'a, T> {
    fn from(value: Result<RwLockReadGuard<'a, T>, PoisonError>) -> Self {
        Self {
            inner: value
        }
    }
}
impl<'a, T> Display for ReadGuard<'a, T> where T: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(e) => {
                write!(f, "(Poisioned: '{e}'")   
            }
        }
    }
}
impl<'a, T> Debug for ReadGuard<'a, T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(e) => {
                write!(f, "(Poisoned: '{e}'")
            }
        }
    }
}
impl<'a, T> PartialEq for ReadGuard<'a, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        match (self.inner.as_deref(), other.inner.as_deref()) {
            (Ok(a), Ok(b)) => a.eq(b),
            _ => false
        }
    }
}
impl<'a, T> PartialEq<T> for ReadGuard<'a, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        self.inner.as_deref().ok() == Some(other)
    }
}
impl<'a, T> Eq for ReadGuard<'a, T>  where T: PartialEq + Eq { }
impl<'a, T> ReadGuard<'a, T> {
    pub fn access(&'a self) -> Option<&'a T> {
        self.inner.as_deref().ok()
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.as_ref().err()
    }
    pub fn as_ref(&'a self) -> Result<&'a RwLockReadGuard<'a, T>, &'a PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&'a self) -> Result<&'a T, &'a PoisonError> {
        self.inner.as_deref()
    }

    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.err()
    }
    pub fn take_lock(self) -> Option<RwLockReadGuard<'a, T>> {
        self.inner.ok()
    }
    pub fn take(self) -> Result<RwLockReadGuard<'a, T>, PoisonError> {
        self.inner
    }
}

pub struct WriteGuard<'a, T> {
    inner: Result<RwLockWriteGuard<'a, T>, PoisonError>
}
impl<'a, T> From<RwLockWriteGuard<'a, T>> for WriteGuard<'a, T> {
    fn from(value: RwLockWriteGuard<'a, T>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl<'a, T> From<PoisonError> for WriteGuard<'a, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }
    }
}
impl<'a, T> From<Result<RwLockWriteGuard<'a, T>, PoisonError>> for WriteGuard<'a, T> {
    fn from(value: Result<RwLockWriteGuard<'a, T>, PoisonError>) -> Self {
        Self {
            inner: value
        }
    }
}
impl<'a, T> WriteGuard<'a, T> {
    pub fn access(&'a mut self) -> Option<&'a mut T> {
        self.inner.as_deref_mut().ok()
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.as_deref().err()
    }
    pub fn as_ref(&'a mut self) -> Result<&'a mut RwLockWriteGuard<'a, T>, &'a mut PoisonError> {
        self.inner.as_mut()
    }
    pub fn as_deref(&'a mut self) -> Result<&'a mut T, &'a mut PoisonError> {
        self.inner.as_deref_mut()
    }

    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.err()
    }
    pub fn take_lock(self) -> Option<RwLockWriteGuard<'a, T>> {
        self.inner.ok()
    }
    pub fn take(self) -> Result<RwLockWriteGuard<'a, T>, PoisonError> {
        self.inner
    }
}

pub struct OptionReadGuard<'a, T> {
    inner: ReadGuard<'a, Option<T>>
}
impl<'a, T> From<RwLockReadGuard<'a, Option<T>>> for OptionReadGuard<'a, T> {
    fn from(value: RwLockReadGuard<'a, Option<T>>) -> Self {
        Self {
            inner: ReadGuard::from(value)
        }
    }
}
impl<T> From<PoisonError> for OptionReadGuard<'_, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: ReadGuard::from(value)
        }
    }
}
impl<'a, T> From<Result<RwLockReadGuard<'a, Option<T>>, PoisonError>> for OptionReadGuard<'a, T> {
    fn from(value: Result<RwLockReadGuard<'a, Option<T>>, PoisonError>) -> Self {
        Self {
            inner: value.into()
        }
    }
}
impl<'a, T> Display for OptionReadGuard<'a, T> where T: Display + Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(e) => {
                write!(f, "(Poisioned: '{e}'")   
            }
        }
    }
}
impl<'a, T> Debug for OptionReadGuard<'a, T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(e) => {
                write!(f, "(Poisoned: '{e}'")
            }
        }
    }
}
impl<'a, T> PartialEq for OptionReadGuard<'a, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<'a, T> PartialEq<T> for OptionReadGuard<'a, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        match self.inner.as_deref() {
            Ok(v) => v.as_ref() == Some(other),
            Err(_) => false
        }
    }
}
impl<'a, T> Eq for OptionReadGuard<'a, T>  where T: PartialEq + Eq { }
impl<'a, T> OptionReadGuard<'a, T> {
    pub fn access(&'a self) -> Option<&'a T> {
        self.inner.access().map(|x| x.as_ref()).flatten()
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.access_error()
    }
    pub fn as_ref(&'a self) -> Result<&'a RwLockReadGuard<'a, Option<T>>, &'a PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&'a self) -> Result<Option<&'a T>, &'a PoisonError> {
        self.inner.as_deref().map(|x| x.as_ref())
    }
    
    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.take_err()
    }
    pub fn take_lock(self) -> Option<RwLockReadGuard<'a, Option<T>>> {
        self.inner.take_lock()
    }
    pub fn take(self) -> Result<RwLockReadGuard<'a, Option<T>>, PoisonError> {
        self.inner.take()
    }
}

pub struct OptionWriteGuard<'a, T> {
    inner: WriteGuard<'a, Option<T>>
}
impl<'a, T> From<RwLockWriteGuard<'a, Option<T>>> for OptionWriteGuard<'a, T> {
    fn from(value: RwLockWriteGuard<'a, Option<T>>) -> Self {
        Self {
            inner: WriteGuard::from(value)
        }
    }
}
impl<T> From<PoisonError> for OptionWriteGuard<'_, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: WriteGuard::from(value)
        }
    }
}
impl<'a, T> From<Result<RwLockWriteGuard<'a, Option<T>>, PoisonError>> for OptionWriteGuard<'a, T> {
    fn from(value: Result<RwLockWriteGuard<'a, Option<T>>, PoisonError>) -> Self {
        Self {
            inner: value.into()
        }
    }
}
impl<'a, T> OptionWriteGuard<'a, T> {
    pub fn access(&'a mut self) -> Option<&'a mut T> {
        self.inner.access().map(|x| x.as_mut()).flatten()
    }
    pub fn access_error(&'a self) -> Option<&'a PoisonError> {
        self.inner.access_error()
    }
    pub fn as_ref(&'a mut self) -> Result<&'a mut RwLockWriteGuard<'a, Option<T>>, &'a mut PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&'a mut self) -> Result<Option<&'a mut T>, &'a mut PoisonError> {
        self.inner.as_deref().map(|x| x.as_mut())
    }

    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.take_err()
    }
    pub fn take_lock(self) -> Option<RwLockWriteGuard<'a, Option<T>>> {
        self.inner.take_lock()
    }
    pub fn take(self) -> Result<RwLockWriteGuard<'a, Option<T>>, PoisonError> {
        self.inner.take()
    }
}