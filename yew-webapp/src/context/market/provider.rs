use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredictionMarket {
    loaded: bool,
    synced: bool,
    markets: Vec<nostr_minions::nostro2::NostrNote>,
    outcomes: Vec<nostr_minions::nostro2::NostrNote>,
}
impl PredictionMarket {
    pub fn markets(&self) -> Vec<markstr_core::PredictionMarket> {
        let markets = self
            .markets
            .iter()
            .filter_map(|market| {
                // Find the outcome Ids in the market note
                let new_outcomes = market.tags.0.iter().find_map(|tag| {
                    let tag_type = tag.first()?;
                    if tag_type != "outcomes" {
                        return None;
                    }
                    let outcome_a = tag.get(1)?;
                    let outcome_b = tag.get(2)?;
                    Some((outcome_a, outcome_b))
                })?;

                // Find and rebuild the outcomes in the state
                let outcome_a = self.outcomes.iter().find_map(|outcome| {
                    (outcome.id.as_ref() == Some(new_outcomes.0)).then(|| {
                        let outcome = markstr_core::PredictionOutcome::new(
                            outcome.content.clone(),
                            outcome.pubkey.clone(),
                            outcome.created_at as u64,
                            outcome.tags.find_tags("outcome").first()?.chars().next()?,
                        )
                        .ok()?;
                        Some(outcome)
                    })
                })??;
                let outcome_b = self.outcomes.iter().find_map(|outcome| {
                    (outcome.id.as_ref() == Some(new_outcomes.1)).then(|| {
                        let outcome = markstr_core::PredictionOutcome::new(
                            outcome.content.clone(),
                            outcome.pubkey.clone(),
                            outcome.created_at as u64,
                            outcome.tags.find_tags("outcome").first()?.chars().next()?,
                        )
                        .ok()?;
                        Some(outcome)
                    })
                })??;
                // Rebuild the market
                let market = markstr_core::PredictionMarket::new(
                    market.content.clone(),
                    outcome_a.outcome.clone(),
                    outcome_b.outcome.clone(),
                    market.pubkey.clone(),
                    market.created_at as u64,
                );
                let market = market.ok()?;
                Some(market)
            })
            .collect::<Vec<markstr_core::PredictionMarket>>();
        markets
    }
}

pub enum PredictionMarketAction {
    Loaded,
    Synced,
    NewMarket(nostr_minions::nostro2::NostrNote),
    NewOutcome(nostr_minions::nostro2::NostrNote),
}

impl Reducible for PredictionMarket {
    type Action = PredictionMarketAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            PredictionMarketAction::Loaded => std::rc::Rc::new(Self {
                loaded: true,
                synced: self.synced,
                markets: self.markets.clone(),
                outcomes: self.outcomes.clone(),
            }),
            PredictionMarketAction::Synced => std::rc::Rc::new(Self {
                loaded: self.loaded,
                synced: true,
                markets: self.markets.clone(),
                outcomes: self.outcomes.clone(),
            }),
            PredictionMarketAction::NewMarket(market) => {
                let mut markets = self.markets.clone();
                markets.push(market);
                std::rc::Rc::new(Self {
                    loaded: self.loaded,
                    synced: self.synced,
                    markets,
                    outcomes: self.outcomes.clone(),
                })
            }
            PredictionMarketAction::NewOutcome(outcome) => {
                let mut outcomes = self.outcomes.clone();
                outcomes.push(outcome);
                std::rc::Rc::new(Self {
                    loaded: self.loaded,
                    synced: self.synced,
                    markets: self.markets.clone(),
                    outcomes,
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
        outcomes: Vec::new(),
    });
    let relay_ctx = nostr_minions::relay_pool::use_nostr_relay_pool();

    let sub_id = use_state(|| None);

    let relay_ctx_clone = relay_ctx.clone();
    let id_setter = sub_id.setter();
    use_memo((), move |_| {
        // Optmistic subscription to market events and their outcomes
        // TODO: Pull only market events, and then query for outcomes specifically
        let market_filter = nostr_minions::nostro2::NostrSubscription {
            kinds: vec![30986, 30987, 30988].into(),
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
            // We don't care about the wrapper notes, only the inner notes
            let Ok(inner_note) = last_note
                .content
                .parse::<nostr_minions::nostro2::NostrNote>()
            else {
                web_sys::console::error_1(&format!("Failed to parse note: {last_note:?}").into());
                return;
            };
            // Market events are tagged with "outcomes"
            if !inner_note.tags.find_tags("outcomes").is_empty() {
                ctx_dispatcher.dispatch(PredictionMarketAction::NewMarket(inner_note));
            } else if !inner_note.tags.find_tags("outcome").is_empty() {
                // Outcome events are tagged with "outcome"
                ctx_dispatcher.dispatch(PredictionMarketAction::NewOutcome(inner_note));
            }
            // TODO: DO more validation here, to ensure the notes are valid before adding them
            // to the state, as this will cause rerenders.
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
    ctx.markets()
}
