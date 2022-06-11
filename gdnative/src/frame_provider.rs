use ethers::prelude::*;
use gdnative::{
    api::ProjectSettings,
    prelude::*,
    tasks::{Async, AsyncMethod}, export::hint::StringHint,
};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Client, Url,
};

#[derive(NativeClass, Debug, Clone)]
#[inherit(Node)]
#[register_with(Self::register)]
pub struct JsonRpcProvider {
    provider: Provider<Http>,
    url: String
}

impl Middleware for JsonRpcProvider {
    type Error = ProviderError;
    type Provider = Http;
    type Inner = Provider<Http>;

    fn inner(&self) -> &Self::Inner {
        &self.provider
    }
}

#[methods]
impl JsonRpcProvider {
    fn new(_owner: &Node) -> Self {
        let url = "http://localhost:8545".parse::<Url>().unwrap();

        let provider = Self::provider_from(url.clone());

        Self { provider, url: url.to_string() }
    }

    fn provider_from(url: Url) -> Provider<Http> {
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
            Http::new_with_client(url, client);

        Provider::new(http_provider)
    }

    fn register(builder: &ClassBuilder<Self>) {
        builder
            .method("get_accounts", Async::new(GetAccounts))
            .done();
        
        builder
            .property("url")
            .with_hint(StringHint::Placeholder { placeholder: "RPC URL".into() })
            .with_setter(Self::set_url)
            .with_default("http://localhost:8545".into())
            .done();
    }

    #[export]
    fn set_url(&mut self, _owner: TRef<Node>, url: String) {
        self.url = url.clone();
        
        self.provider = Self::provider_from(url.parse().unwrap());
    }
}

/// get accounts from provider.
/// equivalent to "eth_provider" JSON RPC method
struct GetAccounts;

impl AsyncMethod<JsonRpcProvider> for GetAccounts {
    fn spawn_with(&self, spawner: gdnative::tasks::Spawner<'_, JsonRpcProvider>) {
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
