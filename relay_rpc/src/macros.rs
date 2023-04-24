#[macro_export]
macro_rules! impl_managed_newtype {
    ($NewType:ident, $WrappedType:ty, $counter_name:expr) => {
        #[derive(
            Debug,
            Hash,
            Clone,
            PartialEq,
            Eq,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::AsRef,
        )]
        #[serde(transparent)]
        #[as_ref(forward)]
        #[from(forward)]
        pub struct $NewType($crate::managed::ManagedResource<$WrappedType, $NewType>);

        const _: () = {
            use {once_cell::sync::Lazy, opentelemetry::Context};

            static COUNTER: Lazy<Option<opentelemetry::metrics::UpDownCounter<i64>>> =
                Lazy::new(|| {
                    $crate::managed::GLOBAL_METER
                        .lock()
                        .map(|lock| {
                            lock.as_deref()
                                .map(|meter| meter.i64_up_down_counter($counter_name).init())
                        })
                        .unwrap_or(None)
                });

            impl $NewType {
                pub fn new(value: impl Into<Arc<$WrappedType>>) -> Self {
                    Self($crate::managed::ManagedResource::new(value))
                }

                pub fn into_value(self) -> Arc<$WrappedType> {
                    self.0.into_value()
                }
            }

            impl $crate::managed::Counter for $NewType {
                fn inc() {
                    if let Some(counter) = COUNTER.as_ref() {
                        counter.add(&Context::new(), 1, &[]);
                    }
                }

                fn dec() {
                    if let Some(counter) = COUNTER.as_ref() {
                        counter.add(&Context::new(), -1, &[]);
                    }
                }
            }

            impl ::std::ops::Deref for $NewType {
                type Target = $crate::managed::ManagedResource<$WrappedType, $NewType>;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        };
    };
}

/// Macro for implementing default stuff on a newtype.
#[macro_export]
macro_rules! new_type {
    (
        $(#[$outer:meta])*
        $NewType:ident: $(#[$inner:meta])* $InnerType:ty
    ) => {
        #[allow(missing_docs)]
        #[derive(
            Debug,
            Hash,
            Clone,
            PartialEq,
            Eq,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::derive_more::Display,
            ::derive_more::From,
            ::derive_more::AsRef,
        )]
        #[serde(transparent)]
        $(#[$outer])*
        pub struct $NewType($(#[$inner])* $InnerType);

        const _: () = {
            impl $NewType {
                #[allow(missing_docs)]
                pub fn new(val: $InnerType) -> Self {
                    Self(val)
                }

                #[allow(missing_docs)]
                pub fn value(&self) -> &$InnerType {
                    &self.0
                }

                #[allow(missing_docs)]
                pub fn into_value(self) -> $InnerType {
                    self.0
                }
            }
        };
    };
}
