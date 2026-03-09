use {
    crate::{
        auth::{
            did::{combine_did_data, extract_did_data, DidError, DID_METHOD_KEY},
            MULTICODEC_ED25519_BASE,
            MULTICODEC_ED25519_HEADER,
            MULTICODEC_ED25519_LENGTH,
        },
        new_type,
    },
    arrayvec::ArrayVec,
    derive_more::{AsMut, AsRef, From},
    ed25519_dalek::VerifyingKey,
    rand::RngCore,
    serde::{Deserialize, Serialize},
    serde_aux::prelude::deserialize_number_from_string,
    std::{str::FromStr, sync::Arc},
};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ClientIdDecodingError {
    #[error("Invalid issuer multicodec base")]
    Base,

    #[error("Invalid issuer base58")]
    Encoding,

    #[error("Invalid multicodec header")]
    Header,

    #[error("Invalid DID key data: {0}")]
    Did(#[from] DidError),

    #[error("Invalid issuer pubkey length")]
    Length,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("Failed to decode Topic: {0}")]
pub struct TopicDecodingError(TopicDecodingErrorInner);

#[derive(Debug, Clone, thiserror::Error)]
enum TopicDecodingErrorInner {
    #[error("Invalid hex: {0}")]
    InvalidHex(#[from] DecodingError),

    #[error("Unknown kind: {0}")]
    UnknownVersion(u8),

    #[error("Unknown region: {0}")]
    UnknownRegion(u8),
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum DecodingError {
    #[error("Invalid encoding")]
    Encoding,

    #[error("Invalid data length")]
    Length,
}

new_type!(
    #[doc = "Represents the client ID type."]
    #[as_ref(forward)]
    #[from(forward)]
    ClientId: Arc<str>
);

impl ClientId {
    pub fn decode(&self) -> Result<DecodedClientId, ClientIdDecodingError> {
        DecodedClientId::try_from(self.clone())
    }
}

impl From<DecodedClientId> for ClientId {
    fn from(val: DecodedClientId) -> Self {
        Self(val.to_string().into())
    }
}

impl TryFrom<ClientId> for DecodedClientId {
    type Error = ClientIdDecodingError;

    fn try_from(value: ClientId) -> Result<Self, Self::Error> {
        value.as_ref().parse()
    }
}

impl From<DidKey> for ClientId {
    fn from(val: DidKey) -> Self {
        val.0.into()
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, AsRef, AsMut, Serialize, Deserialize)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct DidKey(
    #[serde(with = "crate::serde_helpers::client_id_as_did_key")] pub DecodedClientId,
);

impl From<DidKey> for VerifyingKey {
    fn from(val: DidKey) -> Self {
        val.0.as_public_key()
    }
}

impl From<DecodedClientId> for DidKey {
    fn from(val: DecodedClientId) -> Self {
        Self(val)
    }
}

impl TryFrom<ClientId> for DidKey {
    type Error = ClientIdDecodingError;

    fn try_from(value: ClientId) -> Result<Self, Self::Error> {
        value.decode().map(Self)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, AsRef, AsMut, Serialize, Deserialize)]
#[as_ref(forward)]
#[as_mut(forward)]
pub struct DecodedClientId(pub [u8; MULTICODEC_ED25519_LENGTH]);

impl DecodedClientId {
    #[inline]
    pub fn try_from_did_key(did: &str) -> Result<Self, ClientIdDecodingError> {
        extract_did_data(did, DID_METHOD_KEY)?.parse()
    }

    #[inline]
    pub fn to_did_key(&self) -> String {
        combine_did_data(DID_METHOD_KEY, &self.to_string())
    }

    #[inline]
    pub fn from_key(key: &VerifyingKey) -> Self {
        Self(*key.as_bytes())
    }

    #[inline]
    pub fn as_public_key(&self) -> VerifyingKey {
        // We know that the length is correct, so we can just unwrap.
        VerifyingKey::from_bytes(&self.0).unwrap()
    }
}

impl From<VerifyingKey> for DecodedClientId {
    fn from(key: VerifyingKey) -> Self {
        Self::from_key(&key)
    }
}

impl From<DecodedClientId> for VerifyingKey {
    fn from(val: DecodedClientId) -> Self {
        val.as_public_key()
    }
}

impl From<DidKey> for DecodedClientId {
    fn from(val: DidKey) -> Self {
        val.0
    }
}

impl FromStr for DecodedClientId {
    type Err = ClientIdDecodingError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        const TOTAL_DECODED_LENGTH: usize =
            MULTICODEC_ED25519_HEADER.len() + MULTICODEC_ED25519_LENGTH;

        let stripped = val
            .strip_prefix(MULTICODEC_ED25519_BASE)
            .ok_or(ClientIdDecodingError::Base)?;

        let mut decoded: [u8; TOTAL_DECODED_LENGTH] = [0; TOTAL_DECODED_LENGTH];

        let decoded_len = bs58::decode(stripped)
            .into(&mut decoded)
            .map_err(|_| ClientIdDecodingError::Encoding)?;

        if decoded_len != TOTAL_DECODED_LENGTH {
            return Err(ClientIdDecodingError::Length);
        }

        let pub_key = decoded
            .strip_prefix(&MULTICODEC_ED25519_HEADER)
            .ok_or(ClientIdDecodingError::Header)?;

        let mut data = Self::default();
        data.0.copy_from_slice(pub_key);

        Ok(data)
    }
}

impl std::fmt::Display for DecodedClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const PREFIX_LEN: usize = MULTICODEC_ED25519_HEADER.len();
        const TOTAL_LEN: usize = MULTICODEC_ED25519_LENGTH + PREFIX_LEN;

        let mut prefixed_data: [u8; TOTAL_LEN] = [0; TOTAL_LEN];
        prefixed_data[..PREFIX_LEN].copy_from_slice(&MULTICODEC_ED25519_HEADER);
        prefixed_data[PREFIX_LEN..].copy_from_slice(&self.0);

        let encoded_data = bs58::encode(prefixed_data).into_string();

        write!(f, "{MULTICODEC_ED25519_BASE}{encoded_data}")
    }
}

new_type!(
    #[doc = "Represents the topic type."]
    #[as_ref(forward)]
    #[from(forward)]
    Topic: Arc<str>
);

impl Topic {
    pub fn decode(&self) -> Result<DecodedTopic, TopicDecodingError> {
        self.0.parse()
    }

    pub fn generate() -> Self {
        Self::from(DecodedTopic::generate())
    }
}

new_type!(
    #[doc = "Represents the subscription ID type."]
    #[as_ref(forward)]
    #[from(forward)]
    SubscriptionId: Arc<str>
);

new_type!(
    #[doc = "Represents the auth token subject type."]
    #[as_ref(forward)]
    #[from(forward)]
    AuthSubject: Arc<str>
);

new_type!(
    #[doc = "Represents the message ID type."]
    #[derive(Copy)]
    MessageId: #[serde(deserialize_with = "deserialize_number_from_string")] u64
);

impl MessageId {
    /// Minimum allowed value of a [`MessageId`].
    const MIN: Self = Self(1000000000);

    pub(crate) fn validate(&self) -> bool {
        self.0 >= Self::MIN.0
    }

    pub fn is_zero(&self) -> bool {
        // Message ID `0` is used when the client request failed to parse for whatever
        // reason, and the server doesn't know the message ID of that request, but still
        // wants to communicate the error.
        self.0 == 0
    }
}

new_type!(
    #[doc = "Represents the project ID type."]
    #[as_ref(forward)]
    #[from(forward)]
    ProjectId: Arc<str>
);

macro_rules! impl_byte_array_newtype {
    ($NewType:ident, $ParentType:ident, $ByteLength:expr) => {
        #[derive(
            Debug, Default, Clone, Hash, PartialEq, Eq, AsRef, AsMut, Serialize, Deserialize,
        )]
        #[as_ref(forward)]
        #[as_mut(forward)]
        #[serde(transparent)]
        pub struct $NewType(pub [u8; $ByteLength]);

        impl $NewType {
            pub const LENGTH: usize = $ByteLength;

            pub fn generate() -> Self {
                Self(rand::Rng::gen::<[u8; $ByteLength]>(&mut rand::thread_rng()))
            }
        }

        impl FromStr for $NewType {
            type Err = DecodingError;

            fn from_str(val: &str) -> Result<Self, Self::Err> {
                hex_decode_array(val).map(Self)
            }
        }

        impl std::fmt::Display for $NewType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&hex_encode(&self.0))
            }
        }

        const _: () = {
            impl $ParentType {
                pub fn decode(&self) -> Result<$NewType, DecodingError> {
                    $NewType::try_from(self.clone())
                }

                pub fn generate() -> Self {
                    Self::from($NewType::generate())
                }
            }
        };

        impl From<$NewType> for $ParentType {
            fn from(val: $NewType) -> Self {
                Self(val.to_string().into())
            }
        }

        impl TryFrom<$ParentType> for $NewType {
            type Error = DecodingError;

            fn try_from(value: $ParentType) -> Result<Self, Self::Error> {
                value.as_ref().parse()
            }
        }
    };
}

fn hex_encode(bytes: &[u8]) -> String {
    data_encoding::HEXLOWER_PERMISSIVE.encode(bytes)
}

fn hex_decode_array<const N: usize>(val: &str) -> Result<[u8; N], DecodingError>
where
    [u8; N]: Default,
{
    let enc_len = val.len();
    if enc_len == 0 {
        return Err(DecodingError::Length);
    }

    let dec_len = data_encoding::HEXLOWER_PERMISSIVE
        .decode_len(enc_len)
        .map_err(|_| DecodingError::Length)?;

    if dec_len != N {
        return Err(DecodingError::Length);
    }

    let mut data = <[u8; N]>::default();

    data_encoding::HEXLOWER_PERMISSIVE
        .decode_mut(val.as_bytes(), &mut data)
        .map_err(|_| DecodingError::Encoding)?;

    Ok(data)
}

#[derive(Clone, Debug, From, Eq, PartialEq, Hash)]
#[from(forward)]
pub struct DecodedTopic(DecodedTopicInner);

#[derive(Clone, Debug, From, Eq, PartialEq, Hash)]
enum DecodedTopicInner {
    New(TopicId),
    Legacy(LegacyTopicId),
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct LegacyTopicId([u8; 32]);

impl LegacyTopicId {
    fn decode(s: &str) -> Result<Self, TopicDecodingErrorInner> {
        hex_decode_array(s).map(Self).map_err(Into::into)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct TopicId([u8; 18]);

impl TopicId {
    const REGION_BYTE_IDX: usize = 1;
    const VERSION_BYTE_IDX: usize = 0;

    fn decode(s: &str) -> Result<Self, TopicDecodingErrorInner> {
        Self::from_bytes(hex_decode_array(s)?)
    }

    fn from_bytes(bytes: [u8; 18]) -> Result<Self, TopicDecodingErrorInner> {
        use TopicDecodingErrorInner as Error;

        // Validate the `TopicVersion` byte
        let version_byte = bytes[Self::VERSION_BYTE_IDX];
        let _ =
            TopicVersion::try_from_u8(version_byte).ok_or(Error::UnknownVersion(version_byte))?;

        // Validate the `Region` byte
        let region_byte = bytes[Self::REGION_BYTE_IDX];
        let _ = Region::try_from_u8(region_byte).ok_or(Error::UnknownRegion(region_byte))?;

        Ok(Self(bytes))
    }

    fn new(version: TopicVersion, region: Region) -> Self {
        let mut bytes: [u8; 18] = Default::default();

        bytes[Self::VERSION_BYTE_IDX] = version as u8;
        bytes[Self::REGION_BYTE_IDX] = region as u8;

        rand::thread_rng().fill_bytes(&mut bytes[2..]);

        Self(bytes)
    }

    fn version(&self) -> TopicVersion {
        // NOTE: `unwrap` is fine here, as we've validated the `TopicVersion` byte in
        // the constructor
        TopicVersion::try_from_u8(self.0[Self::VERSION_BYTE_IDX]).unwrap()
    }

    fn region(&self) -> Region {
        // NOTE: `unwrap` is fine here, as we've validated the `Region` byte in the
        // constructor
        Region::try_from_u8(self.0[Self::REGION_BYTE_IDX]).unwrap()
    }
}

/// [`Topic`] kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TopicKind {
    /// Legacy topic. Can be either a pairing or a session topic.
    Legacy,

    /// Regular pairing topic.
    Pairing,

    /// WCP-specific pairing topic.
    WcpPairing,
}

#[repr(u8)]
enum TopicVersion {
    PairingV1 = 0,
    WcpPairingV1 = 1,
}

impl TopicVersion {
    fn try_from_u8(val: u8) -> Option<Self> {
        Some(match val {
            0 => Self::PairingV1,
            1 => Self::WcpPairingV1,
            _ => return None,
        })
    }
}

impl From<TopicVersion> for TopicKind {
    fn from(kind: TopicVersion) -> Self {
        match kind {
            TopicVersion::PairingV1 => Self::Pairing,
            TopicVersion::WcpPairingV1 => Self::WcpPairing,
        }
    }
}

/// Region of a Relay deployment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Region {
    Eu = 0,
    Us = 1,
    Ap = 2,
    Sa = 3,
}

impl Region {
    fn try_from_u8(val: u8) -> Option<Self> {
        Some(match val {
            0 => Self::Eu,
            1 => Self::Us,
            2 => Self::Ap,
            3 => Self::Sa,
            _ => return None,
        })
    }
}

impl FromStr for DecodedTopic {
    type Err = TopicDecodingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::decode(s).map_err(TopicDecodingError)
    }
}

impl DecodedTopic {
    /// Generates a new [Legacy](`TopicKind::Legacy`) [`Topic`].
    pub fn generate() -> Self {
        let bytes: [u8; 32] = rand::Rng::gen(&mut rand::thread_rng());

        Self(LegacyTopicId(bytes).into())
    }

    /// Generates a new [Pairing](`TopicKind::Pairing`) [`Topic`].
    pub fn pairing(region: Region) -> Self {
        TopicId::new(TopicVersion::PairingV1, region).into()
    }

    /// Generates a new [WCP Pairing](`TopicKind::WcpPairing`) [`Topic`].
    pub fn wcp_pairing(region: Region) -> Self {
        TopicId::new(TopicVersion::WcpPairingV1, region).into()
    }

    fn decode(s: &str) -> Result<Self, TopicDecodingErrorInner> {
        // Legacy topic ID
        if s.len() == 64 {
            return LegacyTopicId::decode(s).map(Into::into);
        }

        if s.len() < 2 {
            return Err(DecodingError::Length.into());
        }

        TopicId::decode(s).map(Into::into)
    }

    /// Returns [`TopicKind`] of this [`DecodedTopic`].
    pub fn kind(&self) -> TopicKind {
        match &self.0 {
            DecodedTopicInner::New(topic_id) => topic_id.version().into(),
            DecodedTopicInner::Legacy(_) => TopicKind::Legacy,
        }
    }

    /// Indicates whether this topic is being used for WalletConnect Pay.
    ///
    /// Note: this can give false negatives for [`TopicKind::Legacy`].
    pub fn is_wcp(&self) -> bool {
        match self.kind() {
            TopicKind::Legacy | TopicKind::Pairing => false,
            TopicKind::WcpPairing => true,
        }
    }

    /// Returns the [`Region`] the [`Topic`] is being handled by.
    ///
    /// `None` for [`TopicKind::Legacy`].
    pub fn region(&self) -> Option<Region> {
        match &self.0 {
            DecodedTopicInner::New(topic_id) => Some(topic_id.region()),
            DecodedTopicInner::Legacy(_) => None,
        }
    }

    /// Returns the underlying bytes of this [`DecodedTopic`].
    pub fn bytes(&self) -> &[u8] {
        match &self.0 {
            DecodedTopicInner::Legacy(topic_id) => &topic_id.0,
            DecodedTopicInner::New(topic_id) => &topic_id.0,
        }
    }

    /// Converts this [`DecodedTopic`] into a [`Vec`] of the underlying bytes.
    pub fn to_bytes(self) -> Vec<u8> {
        self.bytes().to_vec()
    }
}

impl std::fmt::Display for DecodedTopic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&hex_encode(self.bytes()))
    }
}

impl AsRef<[u8]> for DecodedTopic {
    fn as_ref(&self) -> &[u8] {
        self.bytes()
    }
}

impl From<DecodedTopic> for Topic {
    fn from(val: DecodedTopic) -> Self {
        Self(val.to_string().into())
    }
}

impl TryFrom<Topic> for DecodedTopic {
    type Error = TopicDecodingError;

    fn try_from(value: Topic) -> Result<Self, Self::Error> {
        value.as_ref().parse()
    }
}

impl Serialize for DecodedTopic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DecodedTopic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = ArrayVec::<u8, 32>::deserialize(deserializer)?;

        let invalid_length_error = || serde::de::Error::custom("invalid length");

        match bytes.len() {
            18 => {
                let mut buf: [u8; 18] = Default::default();
                buf.copy_from_slice(&bytes);
                TopicId::from_bytes(buf)
                    .map(Into::into)
                    .map_err(serde::de::Error::custom)
            }
            32 => bytes
                .into_inner()
                .map(LegacyTopicId)
                .map(Into::into)
                .map_err(|_| invalid_length_error()),

            _ => Err(invalid_length_error()),
        }
    }
}

impl_byte_array_newtype!(DecodedSubscription, SubscriptionId, 32);
impl_byte_array_newtype!(DecodedAuthSubject, AuthSubject, 32);
impl_byte_array_newtype!(DecodedProjectId, ProjectId, 16);

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn client_id_decoding() {
        let client_id_str = "z6MkodHZwneVRShtaLf8JKYkxpDGp1vGZnpGmdBpX8M2exxH";
        let client_id_bin = client_id_str.parse::<DecodedClientId>().unwrap();

        assert_eq!(client_id_str, ClientId::from(client_id_bin).as_ref());

        assert!(matches!(
            "z6MkodHZwne".parse::<DecodedClientId>(),
            Err(ClientIdDecodingError::Length)
        ));
    }

    #[test]
    fn topic_decoding() {
        let topic_str = "85089843cebc89ce5bbffd55377b2e65c8a32c2d0a76742f2d6852b5f531a460";
        let topic_bin = topic_str.parse::<DecodedTopic>().unwrap();

        assert_eq!(topic_str, Topic::from(topic_bin).as_ref());

        assert!("85089843ce".parse::<DecodedTopic>().is_err());
    }

    #[test]
    fn new_topic_format_decoding() {
        let topic = DecodedTopic::decode("000032c2d0a76742f2d6852b5f531a460abc").unwrap();
        assert_eq!(topic.region(), Some(Region::Eu));
        assert_eq!(topic.kind(), TopicKind::Pairing);
        assert!(!topic.is_wcp());

        let topic = DecodedTopic::decode("010132c2d0a76742f2d6852b5f531a460abc").unwrap();
        assert_eq!(topic.region(), Some(Region::Us));
        assert_eq!(topic.kind(), TopicKind::WcpPairing);
        assert!(topic.is_wcp());

        let topic = DecodedTopic::decode("000232c2d0a76742f2d6852b5f531a460abc").unwrap();
        assert_eq!(topic.region(), Some(Region::Ap));
        assert_eq!(topic.kind(), TopicKind::Pairing);
        assert!(!topic.is_wcp());

        let topic = DecodedTopic::decode("010332c2d0a76742f2d6852b5f531a460abc").unwrap();
        assert_eq!(topic.region(), Some(Region::Sa));
        assert_eq!(topic.kind(), TopicKind::WcpPairing);
        assert!(topic.is_wcp());

        assert!(DecodedTopic::decode("000432c2d0a76742f2d6852b5f531a460abc").is_err());
        assert!(DecodedTopic::decode("020032c2d0a76742f2d6852b5f531a460abc").is_err());
        assert!(DecodedTopic::decode("000032c2d0a76742f2d6852b5f531a460abcd").is_err());
    }

    #[test]
    fn new_topic_format_constructors() {
        let topic = DecodedTopic::pairing(Region::Eu);
        assert_eq!(topic.region(), Some(Region::Eu));
        assert_eq!(topic.kind(), TopicKind::Pairing);
        assert!(!topic.is_wcp());

        let topic = DecodedTopic::wcp_pairing(Region::Us);
        assert_eq!(topic.region(), Some(Region::Us));
        assert_eq!(topic.kind(), TopicKind::WcpPairing);
        assert!(topic.is_wcp());
    }
}
