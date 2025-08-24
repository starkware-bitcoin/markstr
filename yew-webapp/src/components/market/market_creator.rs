use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Clone, PartialEq, Debug)]
pub struct FormData {
    pub question: String,
    pub outcome_a: String,
    pub outcome_b: String,
    pub settlement_time: String,
    pub description: String,
}

impl Default for FormData {
    fn default() -> Self {
        Self {
            question: String::new(),
            outcome_a: "Yes".to_string(),
            outcome_b: "No".to_string(),
            settlement_time: String::new(),
            description: String::new(),
        }
    }
}

#[derive(Clone, PartialEq, Default)]
pub struct FormErrors {
    pub question: Option<String>,
    pub outcomes: Option<String>,
    pub settlement_time: Option<String>,
}

#[function_component(MarketCreator)]
pub fn market_creator() -> Html {
    let nostr_id = nostr_minions::key_manager::use_nostr_key();
    let relay_ctx = nostr_minions::relay_pool::use_nostr_relay_pool();
    let form_data = use_state(FormData::default);
    let errors = use_state(FormErrors::default);
    let loading = use_state(|| false);

    // Check permissions (commented for you to implement)
    // let has_permission = use_context::<RoleContext>()
    //     .map(|ctx| ctx.has_permission("create_market"))
    //     .unwrap_or(false);

    // Uncomment and implement permission check
    // if !has_permission {
    //     return html! {
    //         <div class="bg-red-400 border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)] p-6">
    //             <h2 class="text-2xl font-bold mb-4 font-['Space_Grotesk']">{"❌ ACCESS DENIED"}</h2>
    //             <p class="text-lg mb-4">{"Only oracles can create markets."}</p>
    //             <button
    //                 onclick={/* navigate to roles */}
    //                 class="bg-white border-2 border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] px-4 py-2 font-bold hover:transform hover:translate-x-1 hover:translate-y-1 transition-all duration-200"
    //             >
    //                 {"SWITCH TO ORACLE"}
    //             </button>
    //         </div>
    //     };
    // }

    let handle_input_change = {
        let form_data = form_data.clone();
        let errors = errors.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            let name = input.name();
            let value = input.value();

            let mut new_form_data = (*form_data).clone();
            match name.as_str() {
                "question" => new_form_data.question = value,
                "description" => new_form_data.description = value,
                "settlementTime" => new_form_data.settlement_time = value,
                _ => {}
            }
            form_data.set(new_form_data);

            // Clear error when user starts typing
            let mut new_errors = (*errors).clone();
            match name.as_str() {
                "question" => new_errors.question = None,
                "settlementTime" => new_errors.settlement_time = None,
                _ => {}
            }
            errors.set(new_errors);
        })
    };

    let handle_outcome_change = {
        let form_data = form_data.clone();
        Callback::from(move |(outcome, value): (char, String)| {
            let mut new_form_data = (*form_data).clone();
            if outcome == 'A' {
                new_form_data.outcome_a = value;
            } else {
                new_form_data.outcome_b = value;
            }
            form_data.set(new_form_data);
        })
    };

    let validate_form = {
        let form_data = form_data.clone();
        let errors = errors.clone();
        Callback::from(move |_| {
            let mut new_errors = FormErrors::default();
            let data = &*form_data;

            if data.question.trim().is_empty() {
                new_errors.question = Some("Question is required".to_string());
            }

            if data.settlement_time.is_empty() {
                new_errors.settlement_time = Some("Settlement time is required".to_string());
            } else {
                // Add datetime validation logic here
                // let settlement_time = data.settlement_time.parse::<u64>().unwrap();
                // if data.settlement_time <= web_sys::js_sys::Date::new_0().to_string() {
                //     new_errors.settlement_time =
                //         Some("Settlement time must be in the future".to_string());
                // }
            }

            if data.outcome_a.trim().is_empty() || data.outcome_b.trim().is_empty() {
                new_errors.outcomes = Some("Outcomes must be filled".to_string());
            }

            let is_valid = new_errors.question.is_none()
                && new_errors.settlement_time.is_none()
                && new_errors.outcomes.is_none();

            errors.set(new_errors);
            is_valid
        })
    };

    let handle_submit = {
        let validate_form = validate_form.clone();
        let loading = loading.clone();
        let form_data = form_data.clone();
        let nostr_id = nostr_id.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if !validate_form.emit(()) {
                return;
            }
            let Some(nostr_key) = nostr_id.as_ref() else {
                return;
            };

            loading.set(true);

            // Implement market creation logic here
            let settlement_time = web_sys::js_sys::Date::parse(&form_data.settlement_time);
            // we build a market with two outcomes
            let Ok(market) = markstr_core::PredictionMarket::new(
                form_data.question.clone(),
                form_data.outcome_a.clone(),
                form_data.outcome_b.clone(),
                nostr_key.public_key(),
                settlement_time.trunc() as u64,
            ) else {
                loading.set(false);
                return;
            };

            // we build a corresponding nostr market event
            let mut market_event = nostr_minions::nostro2::NostrNote {
                content: market.question.clone(),
                kind: 42,
                created_at: settlement_time.trunc() as i64,
                pubkey: nostr_key.public_key(),
                ..Default::default()
            };
            market_event.tags.0.push(vec![
                "outcomes".to_string(),
                market.outcome_a.nostr_id(),
                market.outcome_b.nostr_id(),
            ]);
            market_event
                .serialize_id()
                .expect("Failed to serialize market event");
            // we check that the market event id is the same as the market id
            // if the note was not serialized correctly, this will fail
            assert!(Some(&market.market_id) == market_event.id.as_ref());

            // we sign the market event with the nostr key
            nostr_key
                .sign_note(&mut market_event)
                .expect("Failed to sign market event");

            // we need a wrapper event to publish the market event to the relay network
            // because relays mostly only accept events with current timestamps
            // the market event should be saved by interested parties,
            // allowing th oracle to "close" the bet by replacing the event.
            let mut wrapper_market_event = nostr_minions::nostro2::NostrNote {
                content: serde_json::to_string(&market_event).unwrap(),
                kind: 30986,
                ..Default::default()
            };
            wrapper_market_event
                .tags
                .add_parameter_tag(&market.market_id);
            nostr_key
                .sign_note(&mut wrapper_market_event)
                .expect("Failed to sign market event");

            // we also need to build the outcome notes, and not sign them yet.
            // we will sign them later, when the bet is settled.
            // these notes have to be wrapped as "rumors" so that they can be
            // published to the relay network.
            let mut outcome_a_note = nostr_minions::nostro2::NostrNote {
                content: market.outcome_a.outcome.clone(),
                kind: 42,
                created_at: market.settlement_timestamp as i64,
                pubkey: nostr_key.public_key(),
                ..Default::default()
            };
            outcome_a_note
                .tags
                .0
                .push(vec!["outcome".to_string(), 'A'.to_string()]);
            outcome_a_note
                .serialize_id()
                .expect("Failed to serialize outcome a note");

            let mut outcome_b_note = nostr_minions::nostro2::NostrNote {
                content: market.outcome_b.outcome.clone(),
                kind: 42,
                created_at: market.settlement_timestamp as i64,
                pubkey: nostr_key.public_key(),
                ..Default::default()
            };
            outcome_b_note
                .tags
                .0
                .push(vec!["outcome".to_string(), 'B'.to_string()]);
            outcome_b_note
                .serialize_id()
                .expect("Failed to serialize outcome b note");

            // we assert that the outcome notes are the same as the market notes (id)
            // same assumption as in the market note, this is to ensure notes can be rebuilt
            // and verified in a client-side application, without relying on the relay network.
            assert!(Some(&market.outcome_a.nostr_id()) == outcome_a_note.id.as_ref());
            assert!(Some(&market.outcome_b.nostr_id()) == outcome_b_note.id.as_ref());

            let mut wrapper_note_a = nostr_minions::nostro2::NostrNote {
                content: serde_json::to_string(&outcome_a_note).unwrap(),
                kind: 30987,
                ..Default::default()
            };
            wrapper_note_a.tags.add_parameter_tag(&market.market_id);
            let mut wrapper_note_b = nostr_minions::nostro2::NostrNote {
                content: serde_json::to_string(&outcome_b_note).unwrap(),
                kind: 30988,
                ..Default::default()
            };
            wrapper_note_b.tags.add_parameter_tag(&market.market_id);

            nostr_key
                .sign_note(&mut wrapper_note_a)
                .expect("Failed to sign outcome a note");
            nostr_key
                .sign_note(&mut wrapper_note_b)
                .expect("Failed to sign outcome b note");

            let _ = relay_ctx.send(wrapper_market_event);
            let _ = relay_ctx.send(wrapper_note_a);
            let _ = relay_ctx.send(wrapper_note_b);

            loading.set(false);
        })
    };

    let handle_cancel = Callback::from(|_| {
        // Implement navigation to home
        // navigate("/");
    });

    let get_min_datetime = || {
        // Implement getting current time + 30 minutes
        let now = web_sys::js_sys::Date::new_0();
        now.set_minutes(now.get_minutes() + 30);
        // format datetime for input
        now.to_string().as_string().unwrap_or_default()
    };

    html! {
        <div class="space-y-6">
            // Header
            <div class="bg-white border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)] p-6">
                <h2 class="text-2xl font-bold mb-2 font-['Space_Grotesk']">{"🏦 CREATE MARKET"}</h2>
                <p class="text-gray-600">
                    {"Create a new prediction market as an oracle"}
                </p>
            </div>

            // Form
            <div class="bg-white border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)] p-6">
                <form onsubmit={handle_submit} class="space-y-6">
                    // Question
                    <div>
                        <label class="block text-lg font-bold mb-2 font-['Space_Grotesk']">
                            {"Market Question *"}
                        </label>
                        <input
                            type="text"
                            name="question"
                            value={form_data.question.clone()}
                            onchange={handle_input_change.clone()}
                            placeholder="e.g., Will Bitcoin reach $100k by end of 2024?"
                            class="w-full p-3 border-2 border-black font-mono text-lg focus:outline-none focus:ring-2 focus:ring-orange-400"
                            maxlength="200"
                        />
                        {
                            if let Some(error) = &errors.question {
                                html! { <p class="text-red-600 text-sm mt-1">{error}</p> }
                            } else {
                                html! {}
                            }
                        }
                        <p class="text-sm text-gray-500 mt-1">
                            {format!("{}/200 characters", form_data.question.len())}
                        </p>
                    </div>

                    // Description
                    <div>
                        <label class="block text-lg font-bold mb-2 font-['Space_Grotesk']">
                            {"Description (Optional)"}
                        </label>
                        <textarea
                            name="description"
                            value={form_data.description.clone()}
                            onchange={handle_input_change.clone()}
                            placeholder="Additional context or rules for the market..."
                            class="w-full p-3 border-2 border-black font-mono text-sm focus:outline-none focus:ring-2 focus:ring-orange-400"
                            rows="3"
                            maxlength="500"
                        />
                        <p class="text-sm text-gray-500 mt-1">
                            {format!("{}/500 characters", form_data.description.len())}
                        </p>
                    </div>

                    // Outcomes
                    <div>
                        <label class="block text-lg font-bold mb-2 font-['Space_Grotesk']">
                            {"Possible Outcomes *"}
                        </label>
                        <div class="flex items-center mb-2">
                            <input
                                type="text"
                                value={form_data.outcome_a.clone()}
                                onchange={
                                    let handle_outcome_change = handle_outcome_change.clone();
                                    Callback::from(move |e: Event| {
                                        let input: HtmlInputElement = e.target_unchecked_into();
                                        handle_outcome_change.emit(('A', input.value()));
                                    })
                                }
                                placeholder="Outcome A"
                                class="flex-1 p-3 border-2 border-black font-mono text-lg focus:outline-none focus:ring-2 focus:ring-orange-400"
                                maxlength="50"
                            />
                            <input
                                type="text"
                                value={form_data.outcome_b.clone()}
                                onchange={
                                    let handle_outcome_change = handle_outcome_change.clone();
                                    Callback::from(move |e: Event| {
                                        let input: HtmlInputElement = e.target_unchecked_into();
                                        handle_outcome_change.emit(('B', input.value()));
                                    })
                                }
                                placeholder="Outcome B"
                                class="flex-1 p-3 border-2 border-black font-mono text-lg focus:outline-none focus:ring-2 focus:ring-orange-400"
                                maxlength="50"
                            />
                        </div>
                        {
                            if let Some(error) = &errors.outcomes {
                                html! { <p class="text-red-600 text-sm mt-1">{error}</p> }
                            } else {
                                html! {}
                            }
                        }
                    </div>

                    // Settlement Time
                    <div>
                        <label class="block text-lg font-bold mb-2 font-['Space_Grotesk']">
                            {"Settlement Time *"}
                        </label>
                        <input
                            type="datetime-local"
                            name="settlementTime"
                            value={form_data.settlement_time.clone()}
                            onchange={handle_input_change.clone()}
                            min={get_min_datetime()}
                            class="w-full p-3 border-2 border-black font-mono text-lg focus:outline-none focus:ring-2 focus:ring-orange-400"
                        />
                        {
                            if let Some(error) = &errors.settlement_time {
                                html! { <p class="text-red-600 text-sm mt-1">{error}</p> }
                            } else {
                                html! {}
                            }
                        }
                        <p class="text-sm text-gray-500 mt-1">
                            {"When the market will be settled and the outcome determined"}
                        </p>
                    </div>

                    // Preview
                    <div class="bg-gray-100 border-2 border-black p-4">
                        <h3 class="text-lg font-bold mb-2 font-['Space_Grotesk']">{"📋 PREVIEW"}</h3>
                        <div class="space-y-2">
                            <div>
                                <strong>{"Question: "}</strong>
                                {
                                    if form_data.question.is_empty() {
                                        "No question set"
                                    } else {
                                        &form_data.question
                                    }
                                }
                            </div>
                            <div class="flex items-center">
                                <strong>{"Outcomes: "}</strong>
                                <span class="ml-2">
                                    {
                                        if form_data.outcome_a.is_empty() {
                                            "No outcomes set"
                                        } else {
                                            &form_data.outcome_a
                                        }
                                    }
                                    {{'/'.to_string()}}
                                    {
                                        if form_data.outcome_b.is_empty() {
                                            "No outcomes set"
                                        } else {
                                            &form_data.outcome_b
                                        }
                                    }
                                </span>
                            </div>
                            <div>
                                <strong>{"Settlement: "}</strong>
                                {
                                    if form_data.settlement_time.is_empty() {
                                        "No time set"
                                    } else {
                                        // Format the datetime string for display
                                        &form_data.settlement_time
                                    }
                                }
                            </div>
                        </div>
                    </div>

                    // Submit Button
                    <div class="flex items-center justify-between">
                        <button
                            type="button"
                            onclick={handle_cancel}
                            class="bg-gray-400 border-2 border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] px-6 py-3 font-bold hover:transform hover:translate-x-1 hover:translate-y-1 transition-all duration-200"
                        >
                            {"CANCEL"}
                        </button>
                        <button
                            type="submit"
                            disabled={*loading}
                            class="bg-orange-400 border-2 border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] px-6 py-3 font-bold hover:transform hover:translate-x-1 hover:translate-y-1 transition-all duration-200 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {
                                if *loading {
                                    "CREATING..."
                                } else {
                                    "CREATE MARKET"
                                }
                            }
                        </button>
                    </div>
                </form>
            </div>
        </div>
    }
}
