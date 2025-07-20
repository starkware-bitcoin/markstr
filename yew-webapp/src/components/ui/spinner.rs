use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoadingSpinnerProps {
    #[prop_or(SpinnerSize::Medium)]
    pub size: SpinnerSize,
    #[prop_or_default]
    pub class: String,
}

#[function_component(LoadingSpinner)]
pub fn loading_spinner(props: &LoadingSpinnerProps) -> Html {
    let spinner_class = format!("{} {}", props.size.class(), props.class);

    html! {
        <div class={spinner_class}>
            <div class="animate-spin rounded-full border-2 border-black border-t-transparent"></div>
        </div>
    }
}

#[derive(PartialEq, Clone)]
pub enum SpinnerSize {
    Small,
    Medium,
    Large,
}

impl SpinnerSize {
    fn class(&self) -> &'static str {
        match self {
            SpinnerSize::Small => "w-4 h-4",
            SpinnerSize::Medium => "w-8 h-8",
            SpinnerSize::Large => "w-12 h-12",
        }
    }
}
