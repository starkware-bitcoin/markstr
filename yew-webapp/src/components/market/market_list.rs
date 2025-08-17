use std::collections::HashMap;
use web_sys::js_sys::Date;
use yew::prelude::*;


#[function_component(BettingPage)]
pub fn betting_page() -> Html {
    html! {
        <div class="space-y-6">
            <div class="p-6 bg-white border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]">
                <h2 class="text-2xl font-bold mb-2 font-[Space_Grotesk]">{"🎯 PLACE BET"}</h2>
                <p class="text-gray-600">
                    {"Create a new prediction market as an oracle"}
                </p>
            </div>
            <MarketList />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct MarketListProps {
    #[prop_or(None)]
    pub status: Option<bool>,
    #[prop_or(None)]
    pub limit: Option<usize>,
}

#[function_component(MarketList)]
pub fn market_list(props: &MarketListProps) -> Html {
    let markets = crate::context::use_market_list();
    let filtered_markets = use_state(Vec::<markstr_core::PredictionMarket>::new);
    web_sys::console::log_1(&format!("Markets: {markets:?}").into());

    // Effect to filter markets
    {
        let filtered_markets = filtered_markets.clone();
        let status = props.status;
        let limit = props.limit;

        use_effect_with(markets.clone(), move |markets| {

            let mut filtered = if let Some(status) = status {
                markets
                    .iter()
                    .filter(|market| market.settled == status)
                    .cloned()
                    .collect()
            } else {
                markets.clone()
            };

            if let Some(limit_val) = limit {
                filtered.truncate(limit_val);
            }
            web_sys::console::log_1(&format!("Filtered markets: {filtered:?}").into());

            filtered_markets.set(filtered);
        });
    }

    // Helper functions
    let get_status_color = |status: bool, bets: &[markstr_core::Bet]| -> &str {
        if status {
            "bg-gray-400"
        } else if bets.is_empty() {
            "bg-blue-400"
        } else if bets.iter().any(|bet| bet.amount > 0) {
            "bg-yellow-400"
        } else {
            "bg-green-400"
        }
        // match status {
        //     "active" => "bg-green-400",
        //     "funded" => "bg-yellow-400",
        //     "settled" => "bg-gray-400",
        //     "created" => "bg-blue-400",
        //     _ => "bg-gray-400",
        // }
    };

    let get_status_text = |status: bool, bets: &[markstr_core::Bet]| -> &str {
        if status {
            "🟢 SETTLED"
        } else if bets.is_empty() {
            "🔵 CREATED"
        } else if bets.iter().any(|bet| bet.amount > 0) {
            "🟡 FUNDED"
        } else {
            "⚫ ACTIVE"
        }
    };

    let format_time_remaining = |end_time: f64| -> String {
        let now = Date::now();
        let remaining = end_time - now;

        if remaining <= 0.0 {
            return "Expired".to_string();
        }

        let days = (remaining / (1000.0 * 60.0 * 60.0 * 24.0)).floor() as i32;
        let hours =
            ((remaining % (1000.0 * 60.0 * 60.0 * 24.0)) / (1000.0 * 60.0 * 60.0)).floor() as i32;

        if days > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}h", hours)
        }
    };

    let calculate_odds = |market: &markstr_core::PredictionMarket| -> HashMap<String, String> {
        let mut odds = HashMap::new();

        // for outcome in &market.outcomes {
        //     let outcome_amount: f64 = market
        //         .bets
        //         .iter()
        //         .filter(|bet| bet.outcome == *outcome)
        //         .map(|bet| bet.amount)
        //         .sum();

        //     let percentage = if market.total_pool > 0.0 {
        //         (outcome_amount / market.total_pool * 100.0)
        //     } else {
        //         0.0
        //     };

        //     odds.insert(outcome.clone(), format!("{:.1}", percentage));
        // }

        odds
    };

    // Render empty state
    if filtered_markets.is_empty() {
        return html! {
            <crate::components::Card class="p-6 text-center">
                <div class="text-4xl mb-4">{"🏪"}</div>
                <p class="text-gray-500 mb-4">{"No markets found"}</p>
                <p class="text-sm text-gray-400">
                    {
                        "No markets available yet"
                        // if let Some(status) = props.status {
                        //     "No markets available yet"
                        // } else {
                        //     &format!("No markets found", props.status)
                        // }
                    }
                </p>
            </crate::components::Card>
        };
    }

    // Render markets
    html! {
        <div class="space-y-4">
            {
                filtered_markets.iter().map(|market| {
                    let odds = calculate_odds(market);
                    let market_id = market.market_id.clone();
                    let bets = [market.bets_a.clone(), market.bets_b.clone()].concat();

                    html! {
                        <crate::components::Card key={market.market_id.clone()} class="p-4">
                            <div class="flex items-start justify-between mb-3">
                                <div class="flex-1">
                                    <h4 class="font-bold text-lg font-['Space_Grotesk'] mb-2">
                                        {&market.question}
                                    </h4>
                                    <div class="flex items-center space-x-4">
                                        <span class={format!("{} px-2 py-1 border border-black text-xs font-bold",
                                            get_status_color(market.settled, &bets))}>
                                            {get_status_text(market.settled, &bets)}
                                        </span>
                                        <span class="text-sm font-mono">
                                            {format!("Pool: {} BTC", market.total_amount)}
                                        </span>
                                        <span class="text-sm">
                                            {format!("⏰ {}", format_time_remaining(market.settlement_timestamp as f64))}
                                        </span>
                                        <span class="text-sm">
                                            {format!("🎯 {} bets", bets.len())}
                                        </span>
                                    </div>
                                </div>
                                <div class="flex space-x-2">
                                    <a href={format!("/betting/{}", market.market_id)}>
                                        <crate::components::Button variant={crate::components::ButtonVariant::Secondary}
                                        size={crate::components::ButtonSize::Small}>
                                            {"VIEW"}
                                        </crate::components::Button>
                                    </a>
                                    {
                                        if !market.settled {
                                            html! {
                                                <a href={format!("/betting/{}", market.market_id)}>
                                                    <crate::components::Button
                                                    variant={crate::components::ButtonVariant::Primary}
                                                    size={crate::components::ButtonSize::Small}>
                                                        {"BET"}
                                                    </crate::components::Button>
                                                </a>
                                            }
                                        } else {
                                            html! {}
                                        }
                                    }
                                </div>
                            </div>

                            // Outcomes
                            <div class="grid grid-cols-1 md:grid-cols-3 gap-2">
                                //{
                                    // market.outcomes.iter().map(|outcome| {
                                    //     let outcome_odds = odds.get(outcome).unwrap_or(&"0.0".to_string()).clone();

                                    //     html! {
                                    //         <div key={outcome.clone()} class="border border-black p-2 bg-white">
                                    //             <div class="flex justify-between items-center">
                                    //                 <span class="font-semibold">{outcome}</span>
                                    //                 <span class="text-sm font-mono">{format!("{}%", outcome_odds)}</span>
                                    //             </div>
                                    //             {
                                    //                 if market.status == "settled" &&
                                    //                    market.winning_outcome.as_ref() == Some(outcome) {
                                    //                     html! {
                                    //                         <span class="text-xs bg-green-400 px-2 py-1 border border-black mt-1 inline-block">
                                    //                             {"🏆 WINNER"}
                                    //                         </span>
                                    //                     }
                                    //                 } else {
                                    //                     html! {}
                                    //                 }
                                    //             }
                                    //         </div>
                                    //     }
                                    // }).collect::<Html>()
                                //}
                            </div>

                            // Additional Info
                            <div class="mt-3 pt-3 border-t border-gray-300">
                                <div class="flex justify-between items-center text-sm text-gray-600">
                                    <span>{format!("Market ID: {}", market.market_id)}</span>
                                    <span>
                                        //{format!("Created: {}",
                                        //    Date::new(&(market.settlement_timestamp).into())
                                        //        .to_locale_date_string("en-US", &web_sys::js_sys::Object::new())
                                        //)}
                                    </span>
                                </div>
                            </div>
                        </crate::components::Card>
                    }
                }).collect::<Html>()
            }
        </div>
    }
}

