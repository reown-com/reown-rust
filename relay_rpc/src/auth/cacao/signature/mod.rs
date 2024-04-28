use {
    self::{
        eip191::{verify_eip191, EIP191},
        get_rpc_url::GetRpcUrl,
    },
    super::{Cacao, CacaoError},
    alloy_primitives::{eip191_hash_message, hex::FromHex, Address, Bytes},
    alloy_provider::{network::Ethereum, ReqwestProvider},
    erc6492::verify_signature,
    serde::{Deserialize, Serialize},
};

pub mod eip191;
pub mod get_rpc_url;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, Hash)]
pub struct Signature {
    pub t: String,
    pub s: String,
}

pub const EIP1271: &str = "eip1271";
pub const EIP6492: &str = "eip6492";

impl Signature {
    pub async fn verify(
        &self,
        cacao: &Cacao,
        provider: Option<&impl GetRpcUrl>,
    ) -> Result<(), CacaoError> {
        let chain_id = cacao.p.chain_id_reference()?;
        let address = cacao.p.address()?;
        let address =
            Address::parse_checksummed(address, None).map_err(CacaoError::AddressInvalid)?;
        let signature = Bytes::from_hex(&cacao.s.s).map_err(|_| CacaoError::Verification)?;
        let message = cacao.siwe_message()?;

        match self.t.as_str() {
            EIP191 => {
                // Technically we can use EIP-6492 to verify EIP-191 signatures as well,
                // but since we know the signature type we can avoid an RPC request.
                verify_eip191(&signature, &address, eip191_hash_message(message))
            }
            EIP1271 | EIP6492 => {
                if let Some(provider) = provider {
                    let provider = ReqwestProvider::<Ethereum>::new_http(
                        provider
                            .get_rpc_url(chain_id)
                            .await
                            .ok_or(CacaoError::ProviderNotAvailable)?,
                    );
                    let result = verify_signature(signature, address, message, provider)
                        .await
                        .map_err(CacaoError::Rpc)?;
                    if result.is_valid() {
                        Ok(())
                    } else {
                        Err(CacaoError::Verification)
                    }
                } else {
                    Err(CacaoError::ProviderNotAvailable)
                }
            }
            _ => Err(CacaoError::UnsupportedSignature),
        }
    }
}
