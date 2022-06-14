use std::path::Path;

use ethers::prelude::*;
use gdnative::{
    api::{ProjectSettings, OS},
    export::hint::StringHint,
    prelude::*,
    tasks::{Async, AsyncMethod},
};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, Url,
};

use crate::{remote_signer::RemoteSignerMiddleware, AsyncExecutorDriver};

#[derive(Debug, Clone)]
enum ActiveProvider {
    JsonRpc,
    LocalWallet,
    RemoteWallet,
}

#[derive(NativeClass, Debug, Clone)]
#[inherit(Node)]
#[register_with(Self::_register)]
pub struct EthersProvider {
    json_rpc_provider: Provider<Http>,
    remote_signer: Option<RemoteSignerMiddleware>,
    local_signer: Option<SignerMiddleware<Provider<Http>, LocalWallet>>,
    url: String,
    chain_id: u64,
    address: Option<Address>,
    active_provider: ActiveProvider,
}

#[methods]
impl EthersProvider {
    fn new(_owner: &Node) -> Self {
        let url = "http://localhost:8545".parse::<Url>().unwrap();

        let json_rpc_provider = Self::_provider_from(url.clone());

        Self {
            json_rpc_provider,
            url: url.to_string(),
            remote_signer: None,
            local_signer: None,
            chain_id: 1,
            address: None,
            active_provider: ActiveProvider::JsonRpc,
        }
    }

    #[export]
    fn _ready(&self, owner: &Node) {
        let async_executor_driver = AsyncExecutorDriver::new_instance();
        owner.add_child(async_executor_driver, true);
    }

    #[export]
    fn connect_local_wallet(&mut self, _owner: &Node, keystore_path: String, password: String) {
        let local_signer = self._local_wallet_from(&keystore_path, &password);
        let address = local_signer.address();
        self.local_signer = Some(local_signer);
        self.address = Some(address);
        self.active_provider = ActiveProvider::LocalWallet;
    }

    #[export]
    fn connect_remote_wallet(&mut self, _owner: &Node, address: Vec<u8>) {
        let address = Address::from_slice(&address);
        self.remote_signer = Some(self._remote_wallet_from(address));
        self.address = Some(address);
        self.active_provider = ActiveProvider::RemoteWallet;
    }

    fn _provider_from(url: Url) -> Provider<Http> {
        let project_settings = ProjectSettings::godot_singleton();
        let project_name = project_settings
            .get_setting("application/config/name")
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(
            header::ORIGIN,
            HeaderValue::from_str(&project_name).unwrap(),
        );

        let client = Client::builder().default_headers(headers).build().unwrap();

        let http_provider = Http::new_with_client(url, client);

        Provider::new(http_provider)
    }

    fn _local_wallet_from(
        &self,
        path: &str,
        password: &str,
    ) -> SignerMiddleware<Provider<Http>, LocalWallet> {
        let wallet = if path.split("user://").count() == 1 {
            let keypath = Path::new(path);
            LocalWallet::decrypt_keystore(keypath, password).unwrap()
        } else {
            let os = OS::godot_singleton();
            let user_data_path = os.get_user_data_dir();
            let path = format!("{}/{}", user_data_path, path);
            let keypath = Path::new(&path);
            LocalWallet::decrypt_keystore(keypath, password).unwrap()
        };

        let provider = self.json_rpc_provider.to_owned();

        SignerMiddleware::new(provider, wallet)
    }

    fn _remote_wallet_from(&self, address: Address) -> RemoteSignerMiddleware {
        RemoteSignerMiddleware::new(self.json_rpc_provider.to_owned(), self.chain_id, address)
    }

    fn _register(builder: &ClassBuilder<Self>) {
        builder.method("get_accounts", Async::new(GetAccounts)).done();

        builder
            .property("url")
            .with_hint(StringHint::Placeholder {
                placeholder: "RPC URL".into(),
            })
            .with_setter(Self::_set_url)
            .with_default("http://localhost:8545".into())
            .done();

        builder.property::<u64>("chain_id").with_default(1).done();
    }

    fn _set_url(&mut self, _owner: TRef<Node>, url: String) {
        self.url = url.clone();

        self.json_rpc_provider = Self::_provider_from(url.parse().unwrap());
    }
}

/// get accounts from provider.
/// equivalent to "eth_provider" JSON RPC method
struct GetAccounts;

impl AsyncMethod<EthersProvider> for GetAccounts {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, EthersProvider>) {
        spawner.spawn(|_ctx, this, _args| {
            let provider = this
                .map(|provider, _owner| provider.json_rpc_provider.clone())
                .unwrap();
            async move {
                let accounts = provider.get_accounts().await.unwrap();

                accounts
                    .iter()
                    .map(|address| {
                        address.to_fixed_bytes().owned_to_variant()
                    })
                    .collect::<Vec<Variant>>()
                    .to_variant()
            }
        })
    }
}
