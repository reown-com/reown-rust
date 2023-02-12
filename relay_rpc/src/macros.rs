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
                #[allow(dead_code, missing_docs)]
                pub fn new(val: $InnerType) -> Self {
                    Self(val)
                }

                #[allow(dead_code, missing_docs)]
                pub fn value(&self) -> &$InnerType {
                    &self.0
                }

                #[allow(dead_code, missing_docs)]
                pub fn into_value(self) -> $InnerType {
                    self.0
                }
            }
        };
    };
}
