use ethers::prelude::*;
use gdnative::{
    api::ProjectSettings,
    prelude::*,
    tasks::{Async, AsyncMethod},
};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, Url,
};

#[derive(NativeClass, Debug, Clone)]
#[inherit(Node)]
#[register_with(Self::register)]
pub struct FrameProvider {
    provider: Provider<Http>,
}

impl Middleware for FrameProvider {
    type Error = ProviderError;
    type Provider = Http;
    type Inner = Provider<Http>;

    fn inner(&self) -> &Self::Inner {
        &self.provider
    }
}

#[methods]
impl FrameProvider {
    fn new(_owner: &Node) -> Self {
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

        let http_provider =
            Http::new_with_client(Url::parse("http://127.0.0.1:1248").unwrap(), client);

        let provider = Provider::new(http_provider);

        Self { provider }
    }

    fn register(builder: &ClassBuilder<Self>) {
        builder
            .method("get_accounts", Async::new(GetAccounts))
            .done();
    }
}

/// get accounts from provider.
/// equivalent to "eth_provider" JSON RPC method
struct GetAccounts;

impl AsyncMethod<FrameProvider> for GetAccounts {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, FrameProvider>) {
        spawner.spawn(|_ctx, this, _args| {
            let provider = this.map(|provider, _owner| provider.clone()).unwrap();
            async move {
                let accounts = provider.get_accounts().await.unwrap();

                accounts
                    .iter()
                    .map(|account| account.to_string())
                    .collect::<Vec<String>>()
                    .to_variant()
            }
        })
    }
}
