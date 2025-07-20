use yew::prelude::*;

use yew_router::prelude::*;

#[derive(Clone, PartialEq, Routable)]
pub enum Route {
    #[at("/")]
    Dashboard,
    #[at("/roles")]
    Roles,
    #[at("/create-market")]
    CreateMarket,
    #[at("/betting")]
    Betting,
    #[at("/oracle")]
    Oracle,
    #[at("/payouts")]
    Payouts,
    #[at("/transactions")]
    Transactions,
    #[not_found]
    #[at("/404")]
    NotFound,
}

use yew::prelude::*;
use yew_router::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LayoutProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Layout)]
pub fn layout(props: &LayoutProps) -> Html {
    let location = use_route::<Route>().unwrap();

    let nav_items = vec![
        NavItem::new(Route::Dashboard, "Dashboard", "📊"),
        NavItem::new(Route::Roles, "Roles", "👥"),
        NavItem::oracle(Route::CreateMarket, "Create Market", "🏦"),
        NavItem::new(Route::Betting, "Betting", "🎯"),
        NavItem::oracle(Route::Oracle, "Oracle", "🔮"),
        NavItem::new(Route::Payouts, "Payouts", "💰"),
        NavItem::new(Route::Transactions, "Transactions", "📝"),
    ];

    let is_active = |route: &Route| route == &location;

    html! {
        <div class="min-h-screen bg-gray-50">
            // Header
            <header class="bg-white border-b-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]">
                <div class="container mx-auto px-4 py-4">
                    <div class="flex items-center justify-between">
                        <h1 class="text-3xl font-bold text-black font-['Space_Grotesk']">{ "MARKSTR" }</h1>
                        <div class="flex items-center space-x-4">
                            <div class="bg-orange-400 px-4 py-2 border-2 border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] font-bold">
                                // {
                                //     match role.current_role {
                                //         Role::Oracle => html!{ "🔮 ORACLE" },
                                //         _ => html!{ format!("👤 {}", role.current_role.to_string().to_uppercase()) }
                                //     }
                                // }
                            </div>
                            <div class="bg-cyan-400 px-4 py-2 border-2 border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] font-bold">
                                { "REGTEST" }
                            </div>
                        </div>
                    </div>
                </div>
            </header>

            // Main layout
            <div class="flex">
                // Sidebar
                <aside class="w-64 bg-white border-r-4 border-black min-h-screen">
                    <nav class="p-4">
                        <ul class="space-y-2">
                            {
                                for nav_items
                                    .into_iter()
                                    // .filter(|item| !item.oracle_only || role.current_role == Role::Oracle)
                                    .map(|item| {
                                        let active = is_active(&item.route);
                                        html! {
                                            <li>
                                                <Link<Route>
                                                    to={item.route.clone()}
                                                    classes={classes!(
                                                    "flex",
                                                    "items-center",
                                                    "space-x-3",
                                                    "p-3",
                                                    "border-2",
                                                    "border-black",
                                                    "font-bold",
                                                    "transition-all",
                                                    "duration-200",
                                                    if active {[
                                                        "bg-yellow-400",
                                                        "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
                                                        "transform",
                                                        "translate-x-1",
                                                        "translate-y-1"].as_slice()
                                                    } else {[
                                                        "bg-white",
                                                        "shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
                                                        "hover:bg-orange-100",
                                                        "hover:transform",
                                                        "hover:translate-x-1",
                                                        "hover:translate-y-1"].as_slice()
                                                    }
                                                )}

                                                >
                                                    <span class="text-xl">{ item.icon }</span>
                                                    <span class="font-['Space_Grotesk']">{ item.label }</span>
                                                </Link<Route>>
                                            </li>
                                        }
                                    })
                            }
                        </ul>
                    </nav>
                </aside>

                // Main Content
                <main class="flex-1 p-6">
                    { for props.children.iter() }
                </main>
            </div>
        </div>
    }
}

struct NavItem {
    route: Route,
    label: &'static str,
    icon: &'static str,
    oracle_only: bool,
}

impl NavItem {
    fn new(route: Route, label: &'static str, icon: &'static str) -> Self {
        Self {
            route,
            label,
            icon,
            oracle_only: false,
        }
    }

    fn oracle(route: Route, label: &'static str, icon: &'static str) -> Self {
        Self {
            route,
            label,
            icon,
            oracle_only: true,
        }
    }
}
