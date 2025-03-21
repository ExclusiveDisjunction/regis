use super::error::PoisonError;

use std::sync::{MutexGuard as StdMutexGuard, RwLockReadGuard, RwLockWriteGuard, Arc, RwLock, Mutex};
use std::fmt::{Debug, Display};

// Room for improvement: Include traits that allow for access and access_error, is_ok, is_err, take_err, take_lock are all provided.

/// An abstraction over the direct result of RwLock<T>::read(), for simplicity.
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
impl<T> From<PoisonError> for ReadGuard<'_, T> {
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
impl<T> Display for ReadGuard<'_, T> where T: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(_) => {
                write!(f, "(Poisioned)")   
            }
        }
    }
}
impl<T> Debug for ReadGuard<'_, T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(e) => {
                write!(f, "(Poisoned: '{e}')")
            }
        }
    }
}
impl<T> PartialEq for ReadGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        match (self.inner.as_deref(), other.inner.as_deref()) {
            (Ok(a), Ok(b)) => a.eq(b),
            _ => false
        }
    }
}
impl<T> PartialEq<T> for ReadGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        self.inner.as_deref().ok() == Some(other)
    }
}
impl<T> Eq for ReadGuard<'_, T>  where T: PartialEq + Eq { }
impl<'a, T> ReadGuard<'a, T> {
    pub fn access(&self) -> Option<&T> {
        self.inner.as_deref().ok()
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.as_ref().err()
    }
    pub fn as_ref(&self) -> Result<&RwLockReadGuard<'a, T>, &PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&self) -> Result<&T, &PoisonError> {
        self.inner.as_deref()
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
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

/// An abstraction over the direct result of RwLock<T>::write(), for simplicity.
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
impl<T> From<PoisonError> for WriteGuard<'_, T> {
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
    pub fn access(&mut self) -> Option<&mut T> {
        self.inner.as_deref_mut().ok()
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.as_deref().err()
    }
    pub fn as_ref(&mut self) -> Result<&mut RwLockWriteGuard<'a, T>, &mut PoisonError> {
        self.inner.as_mut()
    }
    pub fn as_deref(&mut self) -> Result<&mut T, &mut PoisonError> {
        self.inner.as_deref_mut()
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
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

/// An abstraction over the direct result of RwLock<Option<T>>::read(), for simplicity. It will map outputs so that the optional results (in case of error OR empty value) will be Option<...>. 
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
impl<T> Display for OptionReadGuard<'_, T> where T: Display + Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(_) => {
                write!(f, "(Poisioned)")   
            }
        }
    }
}
impl<T> Debug for OptionReadGuard<'_, T> where T: Debug {
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
impl<T> PartialEq for OptionReadGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<T> PartialEq<T> for OptionReadGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        match self.inner.as_deref() {
            Ok(v) => v.as_ref() == Some(other),
            Err(_) => false
        }
    }
}
impl<T> Eq for OptionReadGuard<'_, T>  where T: PartialEq + Eq { }
impl<'a, T> OptionReadGuard<'a, T> {
    pub fn access(&self) -> Option<&T> {
        self.inner.access().and_then(|x| x.as_ref())
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.access_error()
    }
    pub fn as_ref(&self) -> Result<&RwLockReadGuard<'a, Option<T>>, &PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&self) -> Result<Option<&T>, &PoisonError> {
        self.inner.as_deref().map(|x| x.as_ref())
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
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

/// An abstraction over the direct result of RwLock<Option<T>>::write(), for simplicity. It will map outputs so that the optional results (in case of error OR empty value) will be Option<...>. 
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
    pub fn access(&mut self) -> Option<&mut T> {
        self.inner.access().and_then(|x| x.as_mut())
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.access_error()
    }
    pub fn as_ref(&mut self) -> Result<&mut RwLockWriteGuard<'a, Option<T>>, &mut PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_deref(&mut self) -> Result<Option<&mut T>, &mut PoisonError> {
        self.inner.as_deref().map(|x| x.as_mut())
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
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

/// An abstraction over the direct result of Mutex<T>::lock(), for simplicity.
pub struct MutexGuard<'a, T> {
    inner: Result<StdMutexGuard<'a, T>, PoisonError>
}
impl<'a, T> From<StdMutexGuard<'a, T>> for MutexGuard<'a, T> where T: 'a {
    fn from(value: StdMutexGuard<'a, T>) -> Self {
        Self {
            inner: Ok(value)
        }
    }
}
impl<T> From<PoisonError> for MutexGuard<'_, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: Err(value)
        }
    }
}
impl<'a, T> From<Result<StdMutexGuard<'a, T>, PoisonError>> for MutexGuard<'a, T> where T: 'a {
    fn from(value: Result<StdMutexGuard<'a, T>, PoisonError>) -> Self {
        Self {
            inner: value
        }
    }
}   
impl<T> Display for MutexGuard<'_, T> where T: Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => v.fmt(f),
            Err(_) => write!(f, "(Poisoned)")
        }
    }
}
impl<T> Debug for MutexGuard<'_, T> where T: Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => v.fmt(f),
            Err(e) => write!(f, "(Poisoned: '{e}')")
        }
    }
}
impl<T> PartialEq for MutexGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        match (self.inner.as_deref(), other.inner.as_deref()) {
            (Ok(a), Ok(b)) => a.eq(b),
            (_, _) => false
        }
    }
}
impl<T> Eq for MutexGuard<'_, T> where T: PartialEq + Eq {}
impl<T> PartialEq<T> for MutexGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        match self.inner.as_deref() {
            Ok(v) => v.eq(other),
            Err(_) => false
        }   
    }
}
impl<'a, T> MutexGuard<'a, T> where T: 'a {
    pub fn access(&self) -> Option<&T> {
        self.inner.as_deref().ok()
    }
    pub fn access_mut(&mut self) -> Option<&mut T> {
        self.inner.as_deref_mut().ok()
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.as_deref().err()
    }
    pub fn as_ref(&self) -> Result<&StdMutexGuard<'a, T>, &PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_mut(&mut self) -> Result<&mut StdMutexGuard<'a, T>, &mut PoisonError> {
        self.inner.as_mut()
    }
    pub fn as_deref(&self) -> Result<&T, &PoisonError> {
        self.inner.as_deref()
    }
    pub fn as_deref_mut(&mut self) -> Result<&mut T, &mut PoisonError> {
        self.inner.as_deref_mut()
    }

    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }

    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.err()
    }
    pub fn take_lock(self) -> Option<StdMutexGuard<'a, T>> {
        self.inner.ok()
    }
    pub fn take(self) -> Result<StdMutexGuard<'a, T>, PoisonError> {
        self.inner
    }
}

/// An abstraction over the direct result of Mutex<Option<T>>::lock(), for simplicity. It will map outputs so that the optional results (in case of error OR empty value) will be Option<...>. 
pub struct OptionMutexGuard<'a, T>{
    inner: MutexGuard<'a, Option<T>> 
}
impl<'a, T> From<StdMutexGuard<'a, Option<T>>> for OptionMutexGuard<'a, T>{
    fn from(value: StdMutexGuard<'a, Option<T>>) -> Self {
        Self {
            inner: MutexGuard::from(value)
        }
    }
}
impl<T> From<PoisonError> for OptionMutexGuard<'_, T> {
    fn from(value: PoisonError) -> Self {
        Self {
            inner: value.into()
        }
    }
}
impl<'a, T> From<Result<StdMutexGuard<'a, Option<T>>, PoisonError>> for OptionMutexGuard<'a, T>{
    fn from(value: Result<StdMutexGuard<'a, Option<T>>, PoisonError>) -> Self {
        Self {
            inner: MutexGuard::from(value)
        }
    }
}
impl<T> Display for OptionMutexGuard<'_, T> where T: Display + Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(_) => {
                write!(f, "(Poisioned)")   
            }
        }
    }
}
impl<T> Debug for OptionMutexGuard<'_, T> where T: Debug  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.inner.as_deref() {
            Ok(v) => {
                v.fmt(f)
            }
            Err(_) => {
                write!(f, "(Poisoned)")
            }
        }
    }
}
impl<T> PartialEq for OptionMutexGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<T> Eq for OptionMutexGuard<'_, T> where T: PartialEq + Eq {} 
impl<T> PartialEq<T> for OptionMutexGuard<'_, T> where T: PartialEq {
    fn eq(&self, other: &T) -> bool {
        match self.inner.as_deref() {
            Ok(v) => v.as_ref() == Some(other),
            Err(_) => false
        }
    }
}
impl<'a, T> OptionMutexGuard<'a, T> where T: 'a {
    pub fn access(&self) -> Option<&T> {
        self.inner.as_deref().ok().and_then(|x| x.as_ref())
    }
    pub fn access_mut(&mut self) -> Option<&mut T> {
        self.inner.as_deref_mut().ok().and_then(|x| x.as_mut())
    }
    pub fn access_error(&self) -> Option<&PoisonError> {
        self.inner.as_deref().err()
    }
    pub fn as_ref(&self) -> Result<&StdMutexGuard<'a, Option<T>>, &PoisonError> {
        self.inner.as_ref()
    }
    pub fn as_mut(&mut self) -> Result<&mut StdMutexGuard<'a, Option<T>>, &mut PoisonError> {
        self.inner.as_mut()
    }
    pub fn as_deref(&self) -> Result<Option<&T>, &PoisonError> {
        self.inner.as_deref().map(|x| x.as_ref())
    }
    pub fn as_deref_mut(&mut self) -> Result<Option<&mut T>, &mut PoisonError> {
        self.inner.as_deref_mut().map(|x| x.as_mut())
    }

    pub fn take_err(self) -> Option<PoisonError> {
        self.inner.take_err()
    }
    pub fn take_lock(self) -> Option<StdMutexGuard<'a, Option<T>>> {
        self.inner.take_lock()
    }
    pub fn take(self) -> Result<StdMutexGuard<'a, Option<T>>, PoisonError> {
        self.inner.take()
    }
}

/// Holds a reference to a specific type `T`, but does not allow anyone to access it, but functions in this module. Used for safe passage of data of Providers.
pub struct ProtectedAccess<'a, T> {
    data: &'a T
}
impl<'a,T> ProtectedAccess<'a, T> {
    pub fn new(data: &'a T) -> Self {
        Self {
            data
        }
    }

    fn take(&self) -> &'a T {
        self.data
    }
}

/// A global-safe object that stores an instance of some data.
pub trait RwProvider {
    type Data;

    /// Returns data used by other traits in a safe way.
    fn access_raw(&self) -> ProtectedAccess<'_, Arc<RwLock<Self::Data>>>;
}
/// A global-safe object that can be accessed with read and write capabilities. 
pub trait RwProviderAccess : RwProvider {
    /// Forces the value of the provider to be `value`. This will clear any poisioning errors, and will not attempt to retreive the old data.
    fn pass(&self, value: Self::Data) {
        let arc = self.access_raw().take();
        let mut guard = match arc.write() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = value;
        arc.clear_poison();
    }
    /// If the data that this object holds is `Default`, then this will call `pass(Self::Data::default())`.
    fn set_to_default(&self) where Self::Data: Default {
        self.pass(Self::Data::default())
    }
    /// Determines if the data held has been poisoned.
    fn is_poisoned(&self) -> bool {
        self.access_raw().take().is_poisoned()
    }

    /// Gets a lock read guard for the data stored.
    fn access(&self) -> ReadGuard<'_, Self::Data> {
        self.access_raw()
            .take()
            .read()
            .map_err(PoisonError::new)
            .into()
    }
    /// Gets a lock write guard for the data stored.
    fn access_mut(&self) -> WriteGuard<'_, Self::Data> {
        self.access_raw()
            .take()
            .write()
            .map_err(PoisonError::new)
            .into()
    }
}
/// A global-safe object that stores optional data, and can be accessed. Note that the type must implement RwProvider, but the type must be Option<T>. 
pub trait OptionRwProvider<T>: RwProvider<Data = Option<T>> {
    /// Forces the value of the provider to be `value`. This will clear any poisioning errors, and will not attempt to retreive the old data.
    fn pass(&self, value: T) {
        let arc = self.access_raw().take();
        let mut guard = match arc.write() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(value);
        arc.clear_poison();
    }
    /// If the type `T` implements Default, this will set the data stored internally to the default of `T`. 
    fn set_to_default(&self) where T: Default {
        self.pass(T::default())
    }
    /// Determines if the data held has been poisoned.
    fn is_poisoned(&self) -> bool {
        self.access_raw().take().is_poisoned()
    }

    /// Sets the data stored to be `None`. 
    fn reset(&self) {
        let raw = self.access_raw().take();
        match raw.write() {
            Ok(mut v) => *v = None,
            Err(e ) => {
                let mut inner = e.into_inner();
                *inner = None;
                raw.clear_poison();
            }
        }
    }
    /// Determines if there is a data stored within the structure.
    fn is_open(&self) -> bool {
        self.access_raw()
            .take()
            .read()
            .map(|v| v.is_some())
            .ok()
            .unwrap_or(false)
    }

    /// Obtains a read lock guard to the internal data.
    fn access(&self) -> OptionReadGuard<'_, T> {
        self.access_raw()
            .take()
            .read()
            .map_err(PoisonError::new)
            .into()
    }
    /// Obtains a write lock guard to the internal data.
    fn access_mut(&self) -> OptionWriteGuard<'_, T> {
        self.access_raw()
            .take()
            .write()
            .map_err(PoisonError::new)
            .into()
    }
}

/// A global-safe object that stores an instance of some data.
pub trait MutexProvider {
    type Data;

    fn access_raw(&self) -> ProtectedAccess<'_, Arc<Mutex<Self::Data>>>;

    fn is_poisoned(&self) -> bool {
        self.access_raw().take().is_poisoned()
    }
}
/// A global-safe object that can be accessed with read and write capabilities. 
pub trait MutexProviderAccess : MutexProvider {
    fn pass(&self, value: Self::Data) {
        let arc = self.access_raw().take();
        let mut guard = match arc.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = value;
        arc.clear_poison();
    }
    fn set_to_default(&self) where Self::Data: Default {
        self.pass(Self::Data::default())
    }
    fn access(&self) -> MutexGuard<'_, Self::Data> {
        self.access_raw()
            .take()
            .lock()
            .map_err(PoisonError::new)
            .into()
    }
}
/// A global-safe object that stores optional data, and can be accessed.
pub trait OptionMutexProvider<T>: MutexProvider<Data = Option<T>> {
    fn pass(&self, value: T) {
        let arc = self.access_raw().take();
        let mut guard = match arc.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner()
        };

        *guard = Some(value);
        arc.clear_poison();
    }
    fn set_to_default(&self) where T: Default {
        self.pass(T::default())
    }

    fn reset(&self) {
        let raw = self.access_raw().take();
        match raw.lock() {
            Ok(mut v) => *v = None,
            Err(e ) => {
                let mut inner = e.into_inner();
                *inner = None;
                raw.clear_poison();
            }
        }
    }
    fn is_open(&self) -> bool {
        self.access_raw()
            .take()
            .lock()
            .map(|v| v.is_some())
            .ok()
            .unwrap_or(false)
    }

    fn access(&self) -> OptionMutexGuard<'_, T> {
        self.access_raw()
            .take()
            .lock()
            .map_err(PoisonError::new)
            .into()
    }
}