use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct CardProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or(CardColor::White)]
    pub color: CardColor,
    #[prop_or_default]
    pub class: Classes,
}

#[function_component(Card)]
pub fn card(props: &CardProps) -> Html {
    let CardProps {
        children,
        color,
        class,
    } = props;

    // let full_class = format!(
    //     "{} border-4 border-black shadow-[8px_8px_0px_0px_rgba(0,0,0,1)] {}",
    //     color.class(),
    //     class
    // );
    let full_class = classes!(
        color.class(),
        "border-4",
        "border-black",
        "shadow-[8px_8px_0px_0px_rgba(0,0,0,1)]",
        class.clone(),
    );
    html! {
        <div class={full_class}>
            { for children.iter() }
        </div>
    }
}

#[derive(PartialEq, Clone)]
pub enum CardColor {
    White,
    Orange,
    Cyan,
    Yellow,
    Green,
    Red,
    Purple,
    Gray,
}

impl CardColor {
    fn class(&self) -> &'static str {
        match self {
            CardColor::White => "bg-white",
            CardColor::Orange => "bg-orange-400",
            CardColor::Cyan => "bg-cyan-400",
            CardColor::Yellow => "bg-yellow-400",
            CardColor::Green => "bg-green-400",
            CardColor::Red => "bg-red-400",
            CardColor::Purple => "bg-purple-400",
            CardColor::Gray => "bg-gray-400",
        }
    }
}
