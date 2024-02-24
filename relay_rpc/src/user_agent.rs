//! Provides types and parsing of user agent strings.

use {
    serde::{de, Deserialize, Serialize},
    std::{fmt::Display, str::FromStr},
    thiserror::Error as ThisError,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, ThisError, Eq, PartialEq)]
pub enum ParsingError {
    #[error("invalid user agent")]
    UserAgent,

    #[error("invalid protocol")]
    Protocol,

    #[error("invalid sdk")]
    Sdk,

    #[error("invalid id")]
    Id,

    #[error("invalid os")]
    Os,
}

/// Implements user agent string parsing according to the WalletConnect User
/// Agent spec:
/// <https://github.com/WalletConnect/walletconnect-docs/blob/main/docs/specs/core/relay/relay-user-agent.md>
///
/// Parsing doesn't involve `serde` and instead relies only on [`FromStr`] and
/// [`Display`]/[`ToString`] traits, since these will always be parsed to and
/// from strings.
///
/// [`UserAgent`] will only fail to parse an empty string. Otherwise the result
/// will be either a [`ValidUserAgent`] or an unknown user agent string.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum UserAgent {
    Unknown(String),
    ValidUserAgent(ValidUserAgent),
}

/// Represents a valid (parsed) user agent.
///
/// Succeeds in parsing only if all of its parts are successfully parsed.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ValidUserAgent {
    pub protocol: Protocol,
    pub sdk: Sdk,
    pub os: OsInfo,
    pub id: Option<Id>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct OsInfo {
    pub os_family: String,
    pub ua_family: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ProtocolKind {
    WalletConnect,
    Unknown(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Protocol {
    pub kind: ProtocolKind,
    pub version: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum SdkLanguage {
    Js,
    Swift,
    Kotlin,
    CSharp,
    Rust,
    Unknown(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Sdk {
    pub language: SdkLanguage,
    pub version: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Environment {
    Browser,
    ReactNative,
    NodeJs,
    Android,
    Ios,
    Unknown(String),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Id {
    pub environment: Environment,
    pub host: Option<String>,
}

const USER_AGENT_DELIMITER: char = '/';

const PROTOCOL_DELIMITER: char = '-';
const PROTOCOL_WALLETCONNECT: &str = "wc";

const SDK_DELIMITER: char = '-';
const SDK_LANGUAGE_JS: &str = "js";
const SDK_LANGUAGE_SWIFT: &str = "swift";
const SDK_LANGUAGE_KOTLIN: &str = "kotlin";
const SDK_LANGUAGE_CSHARP: &str = "csharp";
const SDK_LANGUAGE_RUST: &str = "rust";

const APP_ID_DELIMITER: char = ':';
const ENV_BROWSER: &str = "browser";
const ENV_REACT_NATIVE: &str = "react-native";
const ENV_NODEJS: &str = "nodejs";
const ENV_ANDROID: &str = "android";
const ENV_IOS: &str = "ios";

const OS_DELIMITER: char = '-';

impl FromStr for OsInfo {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        use {once_cell::sync::Lazy, regex::Regex};

        if value.is_empty() {
            return Err(ParsingError::Os);
        }

        let value = value.to_lowercase();

        static PARSER: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^([^\-]+)(-(.*?))?(-(([\d]+)(.([\d]+))?(.([\d]+))?))$").unwrap()
        });

        let (os_family, ua_family, version) = match PARSER.captures(&value) {
            Some(caps) if caps.get(1).is_some() => {
                let os_family = caps.get(1).unwrap().as_str().to_owned();
                let ua_family = caps.get(3).map(|m| m.as_str().to_owned());
                let version = caps.get(5).map(|m| m.as_str().to_owned());

                (os_family, ua_family, version)
            }

            _ => (value, None, None),
        };

        Ok(Self {
            os_family,
            ua_family,
            version,
        })
    }
}

impl Display for OsInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.os_family)?;

        if let Some(ua_family) = &self.ua_family {
            write!(f, "{OS_DELIMITER}{ua_family}")?;
        }

        if let Some(version) = &self.version {
            write!(f, "{OS_DELIMITER}{version}")?;
        }

        Ok(())
    }
}

impl FromStr for ProtocolKind {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Protocol);
        }

        let parsed = match value {
            PROTOCOL_WALLETCONNECT => Self::WalletConnect,
            _ => Self::Unknown(value.to_owned()),
        };

        Ok(parsed)
    }
}

impl Display for ProtocolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind_str = match self {
            Self::WalletConnect => PROTOCOL_WALLETCONNECT,
            Self::Unknown(kind) => kind,
        };

        f.write_str(kind_str)
    }
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{PROTOCOL_DELIMITER}{}", self.kind, self.version)
    }
}

impl FromStr for Protocol {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Protocol);
        }

        let mut parts = value.splitn(2, PROTOCOL_DELIMITER);
        let kind = parts.next();
        let version = parts.next();

        if let (Some(kind), Some(version)) = (kind, version) {
            Ok(Self {
                kind: kind.parse()?,
                version: version.parse::<u32>().map_err(|_| ParsingError::Protocol)?,
            })
        } else {
            Err(ParsingError::Protocol)
        }
    }
}

impl FromStr for SdkLanguage {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Sdk);
        }

        let parsed = match value {
            SDK_LANGUAGE_JS => Self::Js,
            SDK_LANGUAGE_SWIFT => Self::Swift,
            SDK_LANGUAGE_KOTLIN => Self::Kotlin,
            SDK_LANGUAGE_CSHARP => Self::CSharp,
            SDK_LANGUAGE_RUST => Self::Rust,
            _ => Self::Unknown(value.to_owned()),
        };

        Ok(parsed)
    }
}

impl Display for SdkLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let lang_str = match self {
            Self::Js => SDK_LANGUAGE_JS,
            Self::Swift => SDK_LANGUAGE_SWIFT,
            Self::Kotlin => SDK_LANGUAGE_KOTLIN,
            Self::CSharp => SDK_LANGUAGE_CSHARP,
            Self::Rust => SDK_LANGUAGE_RUST,
            Self::Unknown(lang) => lang,
        };

        f.write_str(lang_str)
    }
}

impl Display for Sdk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{SDK_DELIMITER}{}", self.language, self.version)
    }
}

impl FromStr for Sdk {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Sdk);
        }

        let mut parts = value.splitn(2, SDK_DELIMITER);
        let language = parts.next();
        let version = parts.next();

        if let (Some(language), Some(version)) = (language, version) {
            Ok(Self {
                language: language.parse()?,
                version: version.to_owned(),
            })
        } else {
            Err(ParsingError::Sdk)
        }
    }
}

impl FromStr for Environment {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Id);
        }

        let parsed = match value {
            ENV_BROWSER => Self::Browser,
            ENV_REACT_NATIVE => Self::ReactNative,
            ENV_NODEJS => Self::NodeJs,
            ENV_ANDROID => Self::Android,
            ENV_IOS => Self::Ios,
            _ => Self::Unknown(value.to_owned()),
        };

        Ok(parsed)
    }
}

impl Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let env_str = match self {
            Self::Browser => ENV_BROWSER,
            Self::ReactNative => ENV_REACT_NATIVE,
            Self::NodeJs => ENV_NODEJS,
            Self::Android => ENV_ANDROID,
            Self::Ios => ENV_IOS,
            Self::Unknown(env) => env,
        };

        f.write_str(env_str)
    }
}

impl FromStr for Id {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::Id);
        }

        let mut parts = value.splitn(2, APP_ID_DELIMITER);
        let env = parts.next();
        let host = parts.next();

        match (env, host) {
            (Some(env), host) => Ok(Self {
                environment: env.parse()?,
                host: host.map(ToOwned::to_owned),
            }),

            _ => Err(ParsingError::Id),
        }
    }
}

impl Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(host) = &self.host {
            write!(f, "{}{APP_ID_DELIMITER}{}", self.environment, host)
        } else {
            write!(f, "{}", self.environment)
        }
    }
}

impl Display for UserAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(ua) => f.write_str(ua),
            Self::ValidUserAgent(ua) => write!(f, "{ua}"),
        }
    }
}

impl Display for ValidUserAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use USER_AGENT_DELIMITER as DELIM;

        let Self {
            protocol,
            sdk,
            os,
            id,
        } = self;

        if let Some(id) = id {
            write!(f, "{protocol}{DELIM}{sdk}{DELIM}{os}{DELIM}{id}")
        } else {
            write!(f, "{protocol}{DELIM}{sdk}{DELIM}{os}")
        }
    }
}

impl FromStr for ValidUserAgent {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            return Err(ParsingError::UserAgent);
        }

        let mut parts = value.splitn(4, USER_AGENT_DELIMITER);

        match (parts.next(), parts.next(), parts.next(), parts.next()) {
            (Some(protocol), Some(sdk), Some(os), id) => Ok(Self {
                protocol: protocol.parse()?,
                sdk: sdk.parse()?,
                os: os.parse()?,
                id: id.map(FromStr::from_str).transpose()?,
            }),

            _ => Err(ParsingError::UserAgent),
        }
    }
}

impl FromStr for UserAgent {
    type Err = ParsingError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty() {
            Err(ParsingError::UserAgent)
        } else if let Ok(valid_ua) = value.parse::<ValidUserAgent>() {
            Ok(Self::ValidUserAgent(valid_ua))
        } else {
            Ok(Self::Unknown(value.to_owned()))
        }
    }
}

impl TryFrom<String> for UserAgent {
    type Error = ParsingError;

    /// The difference between this implementation and `FromStr` is that this
    /// one avoids cloning the original user agent string.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(ParsingError::UserAgent)
        } else if let Ok(valid_ua) = value.parse::<ValidUserAgent>() {
            Ok(Self::ValidUserAgent(valid_ua))
        } else {
            Ok(Self::Unknown(value))
        }
    }
}

impl Serialize for UserAgent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UserAgent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        UserAgent::try_from(String::deserialize(deserializer)?)
            .map_err(|_| de::Error::custom("invalid user agent"))
    }
}
