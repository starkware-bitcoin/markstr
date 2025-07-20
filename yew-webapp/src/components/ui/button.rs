use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ButtonProps {
    #[prop_or_default]
    pub children: Children,
    #[prop_or_default]
    pub onclick: Option<Callback<MouseEvent>>,
    #[prop_or(false)]
    pub disabled: bool,
    #[prop_or(ButtonVariant::Primary)]
    pub variant: ButtonVariant,
    #[prop_or(ButtonSize::Medium)]
    pub size: ButtonSize,
    #[prop_or_default]
    pub class: String,
}

#[function_component(Button)]
pub fn button(props: &ButtonProps) -> Html {
    let ButtonProps {
        children,
        onclick,
        disabled,
        variant,
        size,
        class,
    } = props;

    let base_styles = "border-2 border-black font-bold transition-all duration-200 hover:transform hover:translate-x-1 hover:translate-y-1 disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:transform-none";

    let full_class = format!(
        "{} {} {} {}",
        base_styles,
        variant.class(),
        size.class(),
        class
    );

    html! {
        <button
            onclick={onclick.clone()}
            disabled={*disabled}
            class={full_class}
        >
            { for children.iter() }
        </button>
    }
}

#[derive(PartialEq, Clone)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Success,
    Warning,
    Danger,
    Gray,
    White,
}

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "bg-orange-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::Secondary => "bg-cyan-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::Success => "bg-green-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::Warning => "bg-yellow-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::Danger => "bg-red-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::Gray => "bg-gray-400 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
            ButtonVariant::White => "bg-white shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]",
        }
    }
}

#[derive(PartialEq, Clone)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}

impl ButtonSize {
    fn class(&self) -> &'static str {
        match self {
            ButtonSize::Small => "px-3 py-1 text-sm",
            ButtonSize::Medium => "px-4 py-2",
            ButtonSize::Large => "px-6 py-3 text-lg",
        }
    }
}
