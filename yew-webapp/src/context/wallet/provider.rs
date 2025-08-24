use bdk_wallet::template::DescriptorTemplate;
use yew::prelude::*;

pub static BTC_ESPLORA_CLIENT: std::sync::LazyLock<bdk_esplora::esplora_client::AsyncClient> =
    std::sync::LazyLock::new(|| {
        bdk_esplora::esplora_client::Builder::new("https://mutinynet.com/api")
            .build_async()
            .expect("Failed to create BTC Esplora client")
    });

#[derive(Clone, Debug)]
pub struct MarketstrWallet {
    loaded: bool,
    synced: bool,
    btc_wallet: std::sync::Arc<tokio::sync::RwLock<Option<bdk_wallet::Wallet>>>,
    persistor: Option<crate::context::IdbPersister>,
}
impl MarketstrWallet {
    pub fn loaded(&self) -> bool {
        self.loaded
    }
    pub fn synced(&self) -> bool {
        self.synced
    }
    pub async fn load(&self, seed: &[u8; 64]) -> Result<(), web_sys::wasm_bindgen::JsValue> {
        web_sys::console::log_1(&"Loading wallet...".into());
        let mut btc_wallet = self.btc_wallet.write().await;
        if self.loaded() {
            web_sys::console::log_1(&"Wallet already loaded".into());
            return Ok(());
        }

        if let Some(persistor) = &self.persistor {
            match persistor.find_change_set().await {
                Ok(change_set) => {
                    let network = bitcoin::Network::Signet;
                    let xpriv = bitcoin::bip32::Xpriv::new_master(network, seed).map_err(|e| {
                        web_sys::wasm_bindgen::JsValue::from_str(
                            format!("Failed to create xpriv: {e}").as_str(),
                        )
                    })?;
                    let (descriptor, keymap, _) =
                        bdk_wallet::template::Bip86(xpriv, bdk_wallet::KeychainKind::External)
                            .build(network)
                            .expect("Failed to build descriptor");
                    match bdk_wallet::Wallet::load()
                        .keymap(bdk_wallet::KeychainKind::External, keymap)
                        .descriptor(bdk_wallet::KeychainKind::External, Some(descriptor.clone()))
                        .extract_keys()
                        .check_network(network)
                        .load_wallet_no_persist(change_set)
                    {
                        Ok(wallet) => {
                            web_sys::console::log_1(&"BTC wallet loaded from persistor".into());
                            *btc_wallet = wallet;
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to load BTC wallet from persistor: {e}").into(),
                            );
                        }
                    }
                }
                Err(_e) => {
                    let network = bitcoin::network::Network::Signet;
                    let xpriv = bitcoin::bip32::Xpriv::new_master(network, seed).map_err(|e| {
                        web_sys::wasm_bindgen::JsValue::from_str(
                            format!("Failed to create xpriv: {e}").as_str(),
                        )
                    })?;
                    let (descriptor, keymap, _) =
                        bdk_wallet::template::Bip86(xpriv, bdk_wallet::KeychainKind::External)
                            .build(network)
                            .expect("Failed to build descriptor");
                    match bdk_wallet::Wallet::create_single(descriptor.clone())
                        .keymap(bdk_wallet::KeychainKind::External, keymap)
                        .network(network)
                        .create_wallet_no_persist()
                    {
                        Ok(wallet) => {
                            web_sys::console::log_1(&"BTC wallet created".into());
                            *btc_wallet = Some(wallet);
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to create BTC wallet: {e}").into(),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub async fn sync(&self) -> Result<(), web_sys::wasm_bindgen::JsValue> {
        web_sys::console::log_1(&"Syncing wallet...".into());
        use bdk_esplora::EsploraAsyncExt;
        let full_scan_request = self
            .btc_wallet
            .read()
            .await
            .as_ref()
            .ok_or(web_sys::wasm_bindgen::JsValue::from_str("now allet yet"))?
            .start_full_scan();
        web_sys::console::log_1(&"Starting BTC wallet full scan".into());
        match BTC_ESPLORA_CLIENT
            .full_scan(full_scan_request, 12, 12)
            .await
        {
            Ok(full_scan_response) => {
                web_sys::console::log_1(&"BTC wallet full scan completed".into());
                if let Some(wallet) = self.btc_wallet.write().await.as_mut() {
                    if let Err(e) = wallet.apply_update_at(
                        full_scan_response,
                        (web_sys::js_sys::Date::now() / 1000.) as u64,
                    ) {
                        web_sys::console::error_1(
                            &format!("Failed to apply BTC wallet update: {e}").into(),
                        );
                    }
                    if let Some(persistor) = &self.persistor {
                        if let Some(change_set) = wallet.take_staged() {
                            if let Err(e) = persistor.persist_change_set(change_set).await {
                                web_sys::console::error_1(
                                    &format!("Failed to persist BTC wallet update: {e:#?}").into(),
                                );
                            } else {
                                web_sys::console::log_1(&"BTC wallet update persisted".into());
                            }
                        }
                    }
                    web_sys::console::log_1(&format!("Wallet synced: {}", wallet.balance()).into());
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to sync BTC wallet: {e}").into());
            }
        }

        Ok(())
    }
    pub async fn btc_balance(&self) -> Option<bdk_wallet::Balance> {
        self.btc_wallet
            .write()
            .await
            .as_ref()
            .map(|wallet| wallet.balance())
    }
    pub async fn btc_address(&self) -> Option<bitcoin::Address> {
        self.btc_wallet.write().await.as_mut().map(|wallet| {
            wallet
                .reveal_next_address(bdk_wallet::KeychainKind::External)
                .address
        })
    }
    pub async fn transactions(
        &self,
    ) -> Vec<(
        bitcoin::Transaction,
        bdk_wallet::chain::ChainPosition<bdk_wallet::chain::ConfirmationBlockTime>,
    )> {
        let txs = self
            .btc_wallet
            .read()
            .await
            .as_ref()
            .map_or(vec![], |wallet| {
                wallet
                    .transactions()
                    .map(|tx| ((*tx.tx_node.tx).clone(), tx.chain_position))
                    .collect::<Vec<_>>()
            });
        // for tx in &txs {
        //     let Ok(Some(tx_info)) = BTC_ESPLORA_CLIENT.get_tx_info(&tx.0.compute_txid()).await
        //     else {
        //         web_sys::console::error_1(
        //             &format!("Failed to get transaction info for {}", tx.0.compute_txid()).into(),
        //         );
        //         continue;
        //     };
        // }
        txs
    }
    pub async fn send_coins(
        &self,
        address: bitcoin::Address,
        amount: bitcoin::Amount,
    ) -> Result<bitcoin::Transaction, web_sys::wasm_bindgen::JsValue> {
        let mut btc_wallet = self.btc_wallet.write().await;
        if let Some(wallet) = btc_wallet.as_mut() {
            let mut tx_builder = wallet.build_tx();
            tx_builder
                .add_recipient(address, amount)
                .fee_rate(bitcoin::FeeRate::from_sat_per_vb(1).expect("Invalid fee rate"));
            let mut psbt = tx_builder.finish().map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to build PSBT: {e}"))
            })?;
            wallet
                .sign(&mut psbt, bdk_wallet::SignOptions::default())
                .map_err(|e| {
                    web_sys::wasm_bindgen::JsValue::from_str(&format!("Failed to sign PSBT: {e}"))
                })?;
            let tx = psbt.extract_tx().map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!(
                    "Failed to extract transaction: {e}"
                ))
            })?;
            BTC_ESPLORA_CLIENT.broadcast(&tx).await.map_err(|e| {
                web_sys::wasm_bindgen::JsValue::from_str(&format!(
                    "Failed to broadcast transaction: {e}"
                ))
            })?;
            if let Some(persistor) = &self.persistor {
                if let Some(staged) = wallet.take_staged() {
                    if let Err(e) = persistor.persist_change_set(staged).await {
                        web_sys::console::error_1(
                            &format!("Failed to persist BTC wallet update: {e:#?}").into(),
                        );
                    } else {
                        web_sys::console::log_1(&"BTC wallet update persisted".into());
                    }
                }
            }
            web_sys::console::log_1(&format!("Transaction sent: {}", tx.compute_txid()).into());
            Ok(tx)
        } else {
            Err(web_sys::wasm_bindgen::JsValue::from_str(
                "Wallet not loaded",
            ))
        }
    }
}

impl PartialEq for MarketstrWallet {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

pub enum MarketstrWalletAction {
    Loaded,
    Synced,
}

impl Reducible for MarketstrWallet {
    type Action = MarketstrWalletAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            MarketstrWalletAction::Loaded => {
                web_sys::console::log_1(&"Wallet loaded".into());
                std::rc::Rc::new(Self {
                    loaded: true,
                    synced: self.synced,
                    persistor: self.persistor.clone(),
                    btc_wallet: self.btc_wallet.clone(),
                })
            }
            MarketstrWalletAction::Synced => {
                web_sys::console::log_1(&"Wallet synced".into());
                std::rc::Rc::new(Self {
                    loaded: self.loaded,
                    synced: true,
                    btc_wallet: self.btc_wallet.clone(),
                    persistor: self.persistor.clone(),
                })
            }
        }
    }
}

pub type MarketstrWalletStore = UseReducerHandle<MarketstrWallet>;

#[function_component(WalletProvider)]
pub fn language_config_provider(props: &yew::html::ChildrenProps) -> HtmlResult {
    let key_ctx = use_context::<nostr_minions::key_manager::NostrIdStore>()
        .expect("No Nostr key context found");
    let wallet_info = yew::suspense::use_future(|| async move {
        let mut nostr_key = key_ctx.get_nostr_key().await.clone();
        nostr_key.as_mut().and_then(|nostr_key| {
            nostr_key.set_extractable(true);
            let mnemonic = nostr_key
                .mnemonic(nostr_minions::nostro2_signer::Language::English)
                .ok()?;
            nostr_key.set_extractable(false);
            let bdk_key = bdk_wallet::bip39::Mnemonic::parse(&mnemonic).ok()?;
            let xpriv =
                bitcoin::bip32::Xpriv::new_master(bitcoin::Network::Signet, &bdk_key.to_seed(""))
                    .ok()?;
            let (descriptor, keymap, _) =
                bdk_wallet::template::Bip86(xpriv, bdk_wallet::KeychainKind::External)
                    .build(bitcoin::Network::Signet)
                    .ok()?;
            descriptor.sanity_check().ok()?;
            Some((descriptor, keymap))
        })
    })?;
    let wallet = (*wallet_info).as_ref().and_then(|(descriptor, keymap)| {
        let network = bitcoin::Network::Signet;
        bdk_wallet::Wallet::create_single(descriptor.clone())
            .keymap(bdk_wallet::KeychainKind::External, keymap.clone())
            .network(network)
            .create_wallet_no_persist()
            .ok()
    });

    let persistor =
        yew::suspense::use_future(|| async { crate::context::IdbPersister::new().await })?;

    let ctx = use_reducer(|| MarketstrWallet {
        loaded: false,
        synced: false,
        persistor: persistor.clone(),
        btc_wallet: std::sync::Arc::new(tokio::sync::RwLock::new(wallet)),
    });

    Ok(html! {
        <ContextProvider<MarketstrWalletStore> context={ctx}>
            {props.children.clone()}
        </ContextProvider<MarketstrWalletStore>>
    })
}

#[hook]
pub fn use_wallet_loader() -> Option<bool> {
    let wallet_ctx = use_context::<MarketstrWalletStore>().expect("No wallet context found");
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
                ctx_clone.dispatch(MarketstrWalletAction::Loaded);
                return true;
            }
            ctx_clone.loaded()
        } else {
            false
        }
    })
    .ok()?;
    Some(*loaded)
}

#[hook]
pub fn use_wallet_sync() -> Option<bool> {
    let wallet_ctx = use_context::<MarketstrWalletStore>().expect("No wallet context found");
    let ctx_clone = wallet_ctx.clone();
    let synced = yew::suspense::use_future_with(
        (wallet_ctx.loaded, wallet_ctx.synced),
        |_synced| async move {
            if !ctx_clone.synced() {
                ctx_clone.sync().await.ok();
                ctx_clone.dispatch(MarketstrWalletAction::Synced);
            }
            ctx_clone.synced()
        },
    )
    .ok()?;
    Some(*synced)
}

#[hook]
pub fn use_wallet_btc_address() -> Option<bitcoin::Address> {
    let wallet_ctx = use_context::<MarketstrWalletStore>().expect("No wallet context found");
    let ctx_clone = wallet_ctx.clone();
    let address = yew::suspense::use_future_with(
        (wallet_ctx.loaded, wallet_ctx.synced),
        |_loaded| async move { ctx_clone.btc_address().await },
    )
    .ok()?;
    (*address).clone()
}

#[hook]
pub fn use_wallet_btc_balance() -> Option<bdk_wallet::Balance> {
    let wallet_ctx = use_context::<MarketstrWalletStore>().expect("No wallet context found");
    let ctx_clone = wallet_ctx.clone();
    let balance = yew::suspense::use_future_with(
        (wallet_ctx.loaded, wallet_ctx.synced),
        |_loaded| async move { ctx_clone.btc_balance().await },
    )
    .ok()?;
    (*balance).clone()
}

#[hook]
pub fn use_wallet_transactions() -> Vec<(
    bitcoin::Transaction,
    bdk_wallet::chain::ChainPosition<bdk_wallet::chain::ConfirmationBlockTime>,
)> {
    let wallet_ctx = use_context::<MarketstrWalletStore>().expect("No wallet context found");
    let ctx_clone = wallet_ctx.clone();
    let transactions = yew::suspense::use_future_with(
        (wallet_ctx.loaded, wallet_ctx.synced),
        |_loaded| async move { ctx_clone.transactions().await },
    );
    let transactions = match transactions {
        Ok(transactions) => transactions,
        Err(_) => return vec![],
    };
    (*transactions).clone()
}
