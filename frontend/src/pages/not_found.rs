use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct NotFoundProps {}

#[function_component(NotFound)]
pub fn not_found(_props: &NotFoundProps) -> Html {
    html! {
        <div class="not-found-page">
            <h1>{"404 - Page Not Found"}</h1>
            <p>{"The page you're looking for doesn't exist."}</p>
        </div>
    }
} 