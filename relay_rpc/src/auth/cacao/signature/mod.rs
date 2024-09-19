use {
    self::{
        eip1271::{verify_eip1271, EIP1271},
        eip191::{eip191_bytes, verify_eip191, EIP191},
        eip6492::{verify_eip6492, EIP6492},
        get_rpc_url::GetRpcUrl,
    },
    super::{Cacao, CacaoError},
    alloy::primitives::Address,
    serde::{Deserialize, Serialize},
    sha3::{Digest, Keccak256},
    std::str::FromStr,
};

pub mod eip1271;
pub mod eip191;
pub mod eip6492;
pub mod get_rpc_url;

#[cfg(test)]
mod test_helpers;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Signature {
    pub t: String,
    pub s: String,
}

impl Signature {
    pub async fn verify(
        &self,
        cacao: &Cacao,
        provider: Option<&impl GetRpcUrl>,
    ) -> Result<(), CacaoError> {
        let address = cacao.p.address()?;

        let signature = data_encoding::HEXLOWER_PERMISSIVE
            .decode(strip_hex_prefix(&cacao.s.s).as_bytes())
            .map_err(|_| CacaoError::Verification)?;

        let hash = Keccak256::new_with_prefix(eip191_bytes(&cacao.siwe_message()?));

        match self.t.as_str() {
            EIP191 => verify_eip191(
                &signature,
                &address.parse().map_err(CacaoError::AddressNotEip191)?,
                hash,
            ),
            EIP1271 => {
                if let Some(provider) = provider {
                    let chain_id = cacao.p.chain_id_reference()?;
                    let provider = provider.get_rpc_url(chain_id).await;
                    if let Some(provider) = provider {
                        verify_eip1271(
                            signature,
                            Address::from_str(&address).map_err(|_| CacaoError::AddressInvalid)?,
                            &hash.finalize()[..]
                                .try_into()
                                .expect("hash length is 32 bytes"),
                            provider,
                        )
                        .await
                    } else {
                        Err(CacaoError::ProviderNotAvailable)
                    }
                } else {
                    Err(CacaoError::Eip1271NotSupported)
                }
            }
            EIP6492 => {
                if let Some(provider) = provider {
                    let chain_id = cacao.p.chain_id_reference()?;
                    let provider = provider.get_rpc_url(chain_id).await;
                    if let Some(provider) = provider {
                        verify_eip6492(
                            signature,
                            Address::from_str(&address).map_err(|_| CacaoError::AddressInvalid)?,
                            &hash.finalize()[..]
                                .try_into()
                                .expect("hash length is 32 bytes"),
                            provider,
                        )
                        .await
                    } else {
                        Err(CacaoError::ProviderNotAvailable)
                    }
                } else {
                    Err(CacaoError::Eip6492NotSupported)
                }
            }
            _ => Err(CacaoError::UnsupportedSignature),
        }
    }
}

/// Remove the "0x" prefix from a hex string.
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}
