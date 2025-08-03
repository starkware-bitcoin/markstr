use yew::prelude::*;

#[function_component(TransactionsPage)]
pub fn transactions_page() -> Html {
    html! {
        <div class="space-y-6">
            <div class="p-6 bg-white border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]">
                <h2 class="text-2xl font-bold mb-2 font-[Space_Grotesk]">{"📝 TRANSACTION HISTORY"}</h2>
                <p class="text-gray-600">
                    {"View transaction history"}
                </p>
            </div>
            <Transactions />
        </div>
    }
}

#[function_component(Transactions)]
pub fn transactions() -> Html {
    let transactions = crate::context::use_wallet_transactions();

    html! {
        <div class="space-y-5">
            {
                transactions.iter().map(|(tx, _)| {
                    let txid = tx.compute_txid();
                    let amount = tx.output[0].value;
                    let address = bitcoin::Address::from_script(&tx.output[0].script_pubkey, bitcoin::Network::Signet).map(|a| a.to_string()).unwrap_or_default();
                    html! {
                        <crate::components::Card class="p-4 font-['Space_Grotesk'] flex justify-evenly items-center">
                            <div class="flex justify-between items-center w-3/4">
                                <span class="font-semibold">{"Transaction ID:"}</span>
                                <span class="font-mono">{ txid.to_string() }</span>
                            </div>
                            <a href={format!("https://mutinynet.com/tx/{}", txid)} target="_blank">
                                <crate::components::Button 
                                    variant={crate::components::ButtonVariant::Secondary}
                                    size={crate::components::ButtonSize::Small}>
                                    {"VIEW"}
                                </crate::components::Button>
                            </a>
                            // <div class="flex justify-between items-center">
                            //     <span class="font-semibold">{"Amount:"}</span>
                            //     <span class="font-mono">{ format!("{} BTC", amount) }</span>
                            // </div>
                            // <div class="flex justify-between items-center">
                            //     <span class="font-semibold">{"Address:"}</span>
                            //     <span class="font-mono">{ address }</span>
                            // </div>
                        </crate::components::Card>
                    }
                }).collect::<Html>()
            }
        </div>
    }
}
