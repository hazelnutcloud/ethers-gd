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
use serde_json::Value;

use crate::AsyncExecutorDriver;

#[derive(Debug, Clone)]
enum ActiveProvider {
    JsonRpc(Provider<Http>),
    LocalWallet(SignerMiddleware<Provider<Http>, LocalWallet>),
}

#[derive(NativeClass, Debug, Clone)]
#[inherit(Node)]
#[register_with(Self::_register)]
pub struct EthersProvider {
    url: String,
    address: Option<Address>,
    active_provider: Option<ActiveProvider>,
}

#[methods]
impl EthersProvider {
    fn new(_owner: &Node) -> Self {
        let url = "http://localhost:8545".parse::<Url>().unwrap();

        Self {
            url: url.to_string(),
            address: None,
            active_provider: None,
        }
    }

    #[export]
    fn _ready(&mut self, owner: &Node) {
        let async_executor_driver = AsyncExecutorDriver::new_instance();
        owner.add_child(async_executor_driver, true);

        let json_rpc = Self::_provider_from(self.url.parse().unwrap());
        self.active_provider = Some(ActiveProvider::JsonRpc(json_rpc));
    }

    #[export]
    fn connect_local_wallet(&mut self, _owner: &Node, keystore_path: String, password: String) {
        let local_signer = self._local_wallet_from(&keystore_path, &password);
        let address = local_signer.address();
        self.active_provider = Some(ActiveProvider::LocalWallet(local_signer));
        self.address = Some(address);
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

        let provider = match self.active_provider.as_ref().unwrap() {
            ActiveProvider::JsonRpc(json_rpc) => json_rpc.clone(),
            ActiveProvider::LocalWallet(local) => local.provider().clone(),
        };

        SignerMiddleware::new(provider, wallet)
    }

    fn _register(builder: &ClassBuilder<Self>) {
        builder.method("get_accounts", Async::new(GetAccounts)).done();
        builder.method("sign_message", Async::new(SignMessage)).done();
        builder.method("request", Async::new(Request)).done();

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
        self.url = url;
    }
}

/// get accounts from provider.
/// equivalent to "eth_provider" JSON RPC method
struct GetAccounts;

impl AsyncMethod<EthersProvider> for GetAccounts {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, EthersProvider>) {
        spawner.spawn(|_ctx, this, _args| {
            let provider = this.map(|provider, _owner| {
                match provider.active_provider.as_ref().unwrap() {
                    ActiveProvider::JsonRpc(ref json_rpc) => json_rpc.clone(),
                    ActiveProvider::LocalWallet(ref local) => local.provider().clone(),
                }
            }).unwrap();
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

/// sign a simple message
/// sends a "eth_sign" json rpc
struct SignMessage;

impl AsyncMethod<EthersProvider> for SignMessage {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, EthersProvider>) {
        spawner.spawn(|_ctx, this, mut args| {
            let provider = this.map(|provider, _owner| {
                match provider.active_provider.as_ref().unwrap() {
                    ActiveProvider::JsonRpc(ref json_rpc) => json_rpc.clone(),
                    ActiveProvider::LocalWallet(ref local) => local.provider().clone(),
                }
            }).unwrap();
            let msg = args.read::<String>().get().unwrap();
            let address = args.read::<Vec<u8>>().get().unwrap();
            async move {
                let signature = provider.sign(msg.into_bytes(), &Address::from_slice(&address)).await.unwrap();
                signature.to_string().owned_to_variant()
            }
        });
    }
}

/// sends a JSON RPC request
struct Request;

impl AsyncMethod<EthersProvider> for Request {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, EthersProvider>) {
        spawner.spawn(|_ctx, this, mut args| {
            let method = args.read::<String>().get().unwrap();
            let params = args.read::<String>().get().unwrap();

            let params: Vec<Value> = serde_json::from_str(&params).unwrap();
            let provider = this.map(|provider, _owner| {
                match provider.active_provider.as_ref().unwrap() {
                    ActiveProvider::JsonRpc(ref json_rpc) => json_rpc.clone(),
                    ActiveProvider::LocalWallet(ref local) => local.provider().clone(),
                }
            }).unwrap();
            async move {
                let res: String = provider.request(&method, params).await.unwrap();
                res.to_variant()
            }
        })
    }
}