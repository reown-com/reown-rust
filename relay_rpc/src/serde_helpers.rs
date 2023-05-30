pub mod client_id_as_did_key {
    use {
        crate::domain::DecodedClientId,
        serde::{Deserialize, Deserializer, Serialize, Serializer},
    };

    pub fn serialize<S>(data: &DecodedClientId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        data.to_did_key().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DecodedClientId, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        DecodedClientId::try_from_did_key(&String::deserialize(deserializer)?)
            .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use {
        crate::domain::{ClientId, DecodedClientId},
        serde::{Deserialize, Serialize},
    };

    #[test]
    fn client_id_as_did_key() {
        #[derive(Serialize, Deserialize)]
        struct Data {
            #[serde(with = "super::client_id_as_did_key")]
            client_id: DecodedClientId,
        }

        let client_id = ClientId::new("z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".into());

        let serialized = serde_json::to_string(&Data {
            client_id: client_id.decode().unwrap(),
        })
        .unwrap();

        assert_eq!(
            serialized,
            r#"{"client_id":"did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"}"#
        );

        let deserialized: Data = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.client_id, client_id.decode().unwrap(),);
    }
}
