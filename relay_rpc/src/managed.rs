use {
    opentelemetry::metrics::Meter,
    serde::{Deserialize, Serialize},
    std::{
        fmt::{Debug, Display},
        hash::{Hash, Hasher},
        marker::PhantomData,
        sync::{Arc, Mutex},
    },
};

pub static GLOBAL_METER: Mutex<Option<&'static Meter>> = Mutex::new(None);

pub fn init_meter(meter: &'static Meter) {
    *GLOBAL_METER.lock().unwrap() = Some(meter);
}

pub trait Counter {
    fn inc();
    fn dec();
}

struct RefCounter<T: Counter>(PhantomData<T>);

impl<T: Counter> RefCounter<T> {
    fn new() -> Self {
        T::inc();
        Self(PhantomData)
    }
}

impl<T: Counter> Drop for RefCounter<T> {
    fn drop(&mut self) {
        T::dec();
    }
}

pub struct ManagedResource<T: ?Sized, U: Counter> {
    value: Arc<T>,
    rc: Arc<RefCounter<U>>,
}

impl<T, U> ManagedResource<T, U>
where
    T: ?Sized,
    U: Counter,
{
    pub fn new(value: impl Into<Arc<T>>) -> Self {
        Self {
            value: value.into(),
            rc: Arc::new(RefCounter::new()),
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> Arc<T> {
        self.value
    }
}

impl<T, U> Serialize for ManagedResource<T, U>
where
    T: Serialize + ?Sized,
    U: Counter,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.value.serialize(serializer)
    }
}

impl<'de, T, U> Deserialize<'de> for ManagedResource<T, U>
where
    Arc<T>: Deserialize<'de>,
    T: ?Sized,
    U: Counter,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(Arc::<T>::deserialize(deserializer)?))
    }
}

impl<T, U> Hash for ManagedResource<T, U>
where
    T: Hash + ?Sized,
    U: Counter,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T, U> PartialEq for ManagedResource<T, U>
where
    T: PartialEq + ?Sized,
    U: Counter,
{
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T, U> Eq for ManagedResource<T, U>
where
    T: Eq + ?Sized,
    U: Counter,
{
}

impl<T, U> Display for ManagedResource<T, U>
where
    T: Display + ?Sized,
    U: Counter,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<T, U> Debug for ManagedResource<T, U>
where
    T: Debug + ?Sized,
    U: Counter,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.value, f)
    }
}

impl<T, U> AsRef<T> for ManagedResource<T, U>
where
    T: ?Sized,
    U: Counter,
{
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T, U> Clone for ManagedResource<T, U>
where
    T: ?Sized,
    U: Counter,
{
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            rc: self.rc.clone(),
        }
    }
}

impl<T, U> From<&str> for ManagedResource<T, U>
where
    Arc<T>: for<'a> From<&'a str>,
    T: ?Sized,
    U: Counter,
{
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
