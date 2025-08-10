use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredictionMarket {
    loaded: bool,
    synced: bool,
    markets: Vec<nostr_minions::nostro2::NostrNote>,
}
impl PredictionMarket {
    pub fn loaded(&self) -> bool {
        self.loaded
    }
    pub fn synced(&self) -> bool {
        self.synced
    }
}

pub enum PredictionMarketAction {
    Loaded,
    Synced,
    NewMarket(nostr_minions::nostro2::NostrNote),
}

impl Reducible for PredictionMarket {
    type Action = PredictionMarketAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            PredictionMarketAction::Loaded => {
                web_sys::console::log_1(&"Wallet loaded".into());
                std::rc::Rc::new(Self {
                    loaded: true,
                    synced: self.synced,
                    markets: self.markets.clone(),
                })
            }
            PredictionMarketAction::Synced => {
                web_sys::console::log_1(&"Wallet synced".into());
                std::rc::Rc::new(Self {
                    loaded: self.loaded,
                    synced: true,
                    markets: self.markets.clone(),
                })
            }
            PredictionMarketAction::NewMarket(market) => {
                web_sys::console::log_1(&format!("New market: {market:?}").into());
                let mut markets = self.markets.clone();
                markets.push(market);
                std::rc::Rc::new(Self {
                    loaded: self.loaded,
                    synced: self.synced,
                    markets,
                })
            }
        }
    }
}

pub type PredictionMarketStore = UseReducerHandle<PredictionMarket>;

#[function_component(MarketProvider)]
pub fn market_provider(props: &yew::html::ChildrenProps) -> HtmlResult {
    let ctx = use_reducer(|| PredictionMarket {
        loaded: false,
        synced: false,
        markets: Vec::new(),
    });
    let relay_ctx = nostr_minions::relay_pool::use_nostr_relay_pool();
    let nostr_id = nostr_minions::key_manager::use_nostr_key();

    let sub_id = use_state(|| None);

    let relay_ctx_clone = relay_ctx.clone();
    let id_setter = sub_id.setter();
    use_memo((), move |_| {
        let market_filter = nostr_minions::nostro2::NostrSubscription {
            kinds: vec![39812].into(),
            ..Default::default()
        };
        if let nostr_minions::nostro2::NostrClientEvent::Subscribe(_, new_sub_id, ..) =
            relay_ctx_clone.send(market_filter)
        {
            id_setter.set(Some(new_sub_id));
        }
    });

    let ctx_dispatcher = ctx.dispatcher();
    use_effect_with(relay_ctx.relay_events.clone(), move |notes| {
        if let Some(nostr_minions::nostro2::NostrRelayEvent::EndOfSubscription(.., sub_id_notice)) =
            notes.last()
        {
            if Some(sub_id_notice) == sub_id.as_ref() {
                ctx_dispatcher.dispatch(PredictionMarketAction::Loaded);
            }
        }
        || {}
    });

    let ctx_dispatcher = ctx.dispatcher();
    use_effect_with(relay_ctx.unique_notes.clone(), move |notes| {
        let run = || {
            let Some(last_note) = notes.last() else {
                return;
            };
            if last_note.kind != 39812 {
                return;
            }
            if serde_json::from_str::<markstr_core::PredictionMarket>(&last_note.content).is_ok() {
                ctx_dispatcher.dispatch(PredictionMarketAction::NewMarket(last_note.clone()));
            }
        };
        run();
        || {}
    });

    Ok(html! {
        <ContextProvider<PredictionMarketStore> context={ctx}>
            {props.children.clone()}
        </ContextProvider<PredictionMarketStore>>
    })
}

#[hook]
pub fn use_market_list() -> Vec<markstr_core::PredictionMarket> {
    let Some(ctx) = use_context::<PredictionMarketStore>() else {
        return vec![];
    };
    ctx.markets
        .iter()
        .filter_map(|market| {
            serde_json::from_str::<markstr_core::PredictionMarket>(&market.content).ok()
        })
        .collect()
}
