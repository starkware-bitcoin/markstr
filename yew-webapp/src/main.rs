mod components;
mod context;

use yew::prelude::*;

fn main() {
    yew::Renderer::<App>::new().render();
}

#[function_component(App)]
fn app_content() -> Html {
    let relays = vec![
        nostr_minions::relay_pool::UserRelay {
            url: "wss://relay.illuminodes.com".to_string(),
            read: true,
            write: true,
        },
        nostr_minions::relay_pool::UserRelay {
            url: "wss://relay.damus.io".to_string(), // public relay
            read: true,
            write: true,
        },
        nostr_minions::relay_pool::UserRelay {
            url: "wss://relay.thisisfake.broken".to_string(), // broken relay for failure case
            read: true,
            write: true,
        },
    ];

    html! {
        <yew::suspense::Suspense fallback={html! {
            <div class="h-screen w-screen flex items-center justify-center">
            <components::LoadingSpinner />
            </div>
        }}>
        <yew_router::BrowserRouter>
        <nostr_minions::relay_pool::NostrRelayPoolProvider {relays}>
            <nostr_minions::key_manager::NostrIdProvider>
            <context::WalletProvider>
            <context::MarketProvider>
            <components::Layout>
                <KeyCheck>
                    <WalletLoad>
                        <Router />
                    </WalletLoad>
                </KeyCheck>
            </components::Layout>
            </context::MarketProvider>
            </context::WalletProvider>
            </nostr_minions::key_manager::NostrIdProvider>
        </nostr_minions::relay_pool::NostrRelayPoolProvider>
        </yew_router::BrowserRouter>
        </yew::suspense::Suspense>
    }
}

#[function_component(KeyCheck)]
fn key_check(props: &yew::html::ChildrenProps) -> Html {
    let key_ctx = use_context::<nostr_minions::key_manager::NostrIdStore>()
        .expect("No Nostr key context found");
    use_memo((), |()| {
        yew::platform::spawn_local(async move {
            if key_ctx.get_nostr_key().await.is_none() && key_ctx.loaded() {
                let Ok(new_id) =
                    nostr_minions::key_manager::UserIdentity::new_local_identity().await
                else {
                    web_sys::console::error_1(&"Failed to create new local identity".into());
                    return;
                };
                let Some(pubkey) = new_id.get_pubkey().await else {
                    web_sys::console::error_1(
                        &"Failed to get pubkey from new local identity".into(),
                    );
                    return;
                };

                key_ctx.dispatch(nostr_minions::key_manager::NostrIdAction::LoadIdentity(
                    pubkey.to_string(),
                    new_id,
                ))
            }
        });
    });
    html! {
        {props.children.clone()}
    }
}

#[function_component(WalletLoad)]
fn wallet_load(props: &yew::html::ChildrenProps) -> HtmlResult {
    let wallet_ctx =
        use_context::<context::MarketstrWalletStore>().expect("No wallet context found");
    let key_ctx = nostr_minions::key_manager::use_nostr_key();
    let ctx_clone = wallet_ctx.clone();
    let loaded = yew::suspense::use_future_with(key_ctx, |nostr_key| async move {
        if let Some(mut key) = (*nostr_key).clone() {
            key.set_extractable(true);
            let Ok(mnemonic) = key.mnemonic(nostr_minions::nostro2_signer::Language::English)
            else {
                return false;
            };
            key.set_extractable(false);
            let Some(bdk_key) = bdk_wallet::bip39::Mnemonic::parse(&mnemonic).ok() else {
                return false;
            };
            if !ctx_clone.loaded() {
                ctx_clone.load(&bdk_key.to_seed("")).await.ok();
                ctx_clone.sync().await.ok();
                ctx_clone.dispatch(context::MarketstrWalletAction::Loaded);
                return true;
            }
            ctx_clone.loaded()
        } else {
            true
        }
    })?;
    match *loaded {
        true => Ok(props.children.clone()),
        false => Ok(html! {
            <components::LoadingSpinner />
        }),
    }
}

#[function_component(Router)]
fn router() -> Html {
    html! {
        <yew_router::Switch<components::Route> render = { move |switch: components::Route| {
            match switch {
                components::Route::Dashboard => html! {
                    <components::Dashboard />
                },
                components::Route::CreateMarket => html! {
                    <components::MarketCreator />
                },
                components::Route::Betting  => html! {
                    <components::BettingPage />
                },
                _ => html! {
                    <div class="flex flex-1 items-center justify-center">
                        <div class="text-3xl font-bold text-black font-['Space_Grotesk']">{ "404" }</div>
                        <div class="text-xl font-bold text-black font-['Space_Grotesk']">{ "Page not found" }</div>
                    </div>
                }
            }
        }}
        />
    }
}
