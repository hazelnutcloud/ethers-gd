use async_trait::async_trait;
use ethers::prelude::*;
use ethers::providers::Http;
use ethers::types::transaction::{eip2718::TypedTransaction, eip712::Eip712};
use ethers::utils::hash_message;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct RemoteSignerMiddleware {
    provider: Provider<Http>,
    chain_id: u64,
    address: Address,
}

#[derive(Debug, Error)]
pub enum RemoteSignerMiddlewareError {
    #[error(transparent)]
    ProviderError(#[from] ProviderError),

    #[error("error encoding eip712 struct: {0:?}")]
    Eip712Error(String),
}

impl RemoteSignerMiddleware {
    pub fn new(provider: Provider<Http>, chain_id: u64, address: Address) -> RemoteSignerMiddleware {
        Self {
            provider,
            chain_id,
            address,
        }
    }
}

impl Middleware for RemoteSignerMiddleware {
    type Error = ProviderError;
    type Provider = Http;
    type Inner = Provider<Http>;

    fn inner(&self) ->  &Self::Inner {
        &self.provider
    }
}

#[async_trait]
impl Signer for RemoteSignerMiddleware {
    type Error = RemoteSignerMiddlewareError;

    async fn sign_message<S: Send + Sync + AsRef<[u8]>>(
        &self,
        message: S,
    ) -> Result<Signature, Self::Error> {
        let message = message.as_ref();
        let message_hash = hash_message(message);
        let signature = self.provider.request("eth_sign", [message_hash]).await?;
        Ok(signature)
    }

    async fn sign_transaction(&self, tx: &TypedTransaction) -> Result<Signature, Self::Error> {
        let sighash = tx.sighash();
        let signature = self.provider.request("eth_sign", [sighash]).await?;
        Ok(signature)
    }

    async fn sign_typed_data<T: Eip712 + Send + Sync>(
        &self,
        payload: &T,
    ) -> Result<Signature, Self::Error> {
        let payload = payload
            .encode_eip712()
            .map_err(|e| Self::Error::Eip712Error(e.to_string()))?;
        let signature = self
            .provider
            .request("eth_sign", [H256::from(payload)])
            .await?;
        Ok(signature)
    }

    fn address(&self) -> Address {
        self.address
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn with_chain_id<T: Into<u64>>(mut self, chain_id: T) -> Self {
        self.chain_id = chain_id.into();
        self
    }
}
