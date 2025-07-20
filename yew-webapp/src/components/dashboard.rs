use crate::components::Route;
use yew::prelude::*;
use yew_router::components::Link;

#[derive(Clone, PartialEq)]
pub struct Market {
    pub id: String,
    pub question: String,
    pub outcomes: Vec<String>,
    pub status: String,
    pub total_pool: f64,
    pub end_time: i64,
    pub winning_outcome: Option<String>,
    pub bets: Vec<Bet>,
}

#[derive(Clone, PartialEq)]
pub struct Bet {
    pub outcome: String,
    pub amount: f64,
}

#[function_component(Dashboard)]
pub fn dashboard() -> Html {
    let current_role = use_state(|| "user".to_string());
    let balance = crate::context::use_wallet_btc_balance().unwrap_or_default();
    let address = crate::context::use_wallet_btc_address()
        .map(|a| a.to_string())
        .unwrap_or_default();
    crate::context::use_wallet_sync();

    let mock_markets = use_state(|| {
        vec![
            Market {
                id: "market-1".to_string(),
                question: "Will Bitcoin reach $100k by end of 2024?".to_string(),
                outcomes: vec!["Yes".to_string(), "No".to_string()],
                status: "active".to_string(),
                total_pool: 2.5,
                end_time: 1735689600000,
                winning_outcome: None,
                bets: vec![
                    Bet {
                        outcome: "Yes".to_string(),
                        amount: 1.2,
                    },
                    Bet {
                        outcome: "No".to_string(),
                        amount: 1.3,
                    },
                ],
            },
            // Add other markets here similarly...
        ]
    });

    html! {
        <div class="space-y-6">
            <div class="p-6 bg-white border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]">
                <h2 class="text-2xl font-bold mb-2 font-[Space_Grotesk]">
                    { format!("Welcome back, {}!", if *current_role == "oracle" { "🔮 Oracle".to_string() } else { format!("👤 {}", current_role.to_uppercase()) }) }
                </h2>
                <p class="text-gray-600">
                    { if *current_role == "oracle" {
                        "Monitor markets and settle outcomes"
                    } else {
                        "Discover prediction markets and place your bets"
                    } }
                </p>
            </div>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
                <crate::components::Card color={crate::components::CardColor::Cyan} class={classes!("p-6")}>
                    <h3 class="text-xl font-bold mb-4 font-[Space_Grotesk]">{"💰 WALLET"}</h3>
                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Balance:"}</span>
                            <span class="font-mono">{ format!("{} BTC", balance.total()) }</span>
                        </div>
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Pending:"}</span>
                            <span class="font-mono">{ format!("{} BTC", (balance.trusted_pending + balance.untrusted_pending).to_btc()) }</span>
                        </div>
                        <div class="text-xs font-mono bg-black text-white p-2 mt-2 break-all">
                            { &address }
                        </div>
                    </div>
                </crate::components::Card>

                <crate::components::Card color={crate::components::CardColor::Orange} class={classes!("p-6")}>
                    <h3 class="text-xl font-bold mb-4 font-[Space_Grotesk]">{"📊 MARKETS"}</h3>
                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Total:"}</span>
                            <span class="font-mono">{ mock_markets.len() }</span>
                        </div>
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Active:"}</span>
                            <span class="font-mono">{ mock_markets.iter().filter(|m| m.status == "active").count() }</span>
                        </div>
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Settled:"}</span>
                            <span class="font-mono">{ mock_markets.iter().filter(|m| m.status == "settled").count() }</span>
                        </div>
                    </div>
                </crate::components::Card>

                <crate::components::Card color={crate::components::CardColor::Yellow} class={classes!("p-6")}>
                    <h3 class="text-xl font-bold mb-4 font-[Space_Grotesk]">{"📈 VOLUME"}</h3>
                    <div class="space-y-2">
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Total Pool:"}</span>
                            <span class="font-mono">{ format!("{:.2} BTC", mock_markets.iter().map(|m| m.total_pool).sum::<f64>()) }</span>
                        </div>
                        <div class="flex justify-between">
                            <span class="font-semibold">{"Avg Pool:"}</span>
                            <span class="font-mono">{ format!("{:.2} BTC", mock_markets.iter().map(|m| m.total_pool).sum::<f64>() / mock_markets.len() as f64) }</span>
                        </div>
                    </div>
                </crate::components::Card>
            </div>

            <QuickActions current_role={(*current_role).clone()} />
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct DashboardProps {
    pub current_role: String,
}

#[function_component(QuickActions)]
fn quick_actions(DashboardProps { current_role }: &DashboardProps) -> Html {
    html! {
        <div class={classes!("bg-white", "border-4", "border-black", "shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]", "p-6")}>
            <h3 class={classes!("text-xl", "font-bold", "mb-4", "font-['Space_Grotesk']")}>{"⚡ QUICK ACTIONS"}</h3>
            <div class={classes!("grid", "grid-cols-2", "md:grid-cols-4", "gap-4")}>
                {
                    // if current_role == "oracle" {
                        html! {
                            <Link<Route>
                                to={Route::CreateMarket}
                                classes={classes!(
                                    "bg-green-400", "border-2", "border-black",
                                    "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]", "p-4",
                                    "text-center", "font-bold", "hover:transform",
                                    "hover:translate-x-1", "hover:translate-y-1", "transition-all", "duration-200"
                                )}
                            >
                                {"🏦 CREATE MARKET"}
                            </Link<Route>>
                        }
                    // } else {
                    //     html! {}
                    // }
                }
                <Link<Route> to={Route::Betting} classes={classes!(
                    "bg-blue-400", "border-2", "border-black", "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
                    "p-4", "text-center", "font-bold", "hover:transform", "hover:translate-x-1",
                    "hover:translate-y-1", "transition-all", "duration-200"
                )}>{"🎯 PLACE BET"}</Link<Route>>
                <Link<Route> to={Route::Payouts} classes={classes!(
                    "bg-purple-400", "border-2", "border-black", "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
                    "p-4", "text-center", "font-bold", "hover:transform", "hover:translate-x-1",
                    "hover:translate-y-1", "transition-all", "duration-200"
                )}>{"💰 CLAIM PAYOUT"}</Link<Route>>
                <Link<Route> to={Route::Transactions} classes={classes!(
                    "bg-pink-400", "border-2", "border-black", "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
                    "p-4", "text-center", "font-bold", "hover:transform", "hover:translate-x-1",
                    "hover:translate-y-1", "transition-all", "duration-200"
                )}>{"📝 VIEW HISTORY"}</Link<Route>>
            </div>
        </div>
    }
}
