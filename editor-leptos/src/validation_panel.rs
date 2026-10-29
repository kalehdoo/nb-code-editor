use editor_core::ValidationError;
use leptos::prelude::*;
#[component]
pub fn ValidationPanel(
    errors: ReadSignal<Vec<ValidationError>>,
    validated: ReadSignal<bool>,
) -> impl IntoView {
    view! { <div class="nb-validation" role="status" aria-live="polite">{move || if !validated.get() { Vec::new() } else if errors.get().is_empty() { vec!["Syntax valid.".to_owned()] } else { errors.get().iter().map(|e| format!("{}{}", e.line.map(|l| format!("Line {l}{}: ", e.column.map(|c| format!(", column {c}")).unwrap_or_default())).unwrap_or_default(), e.message)).collect() }.into_iter().map(|message| view! { <div>{message}</div> }).collect_view()}</div> }
}
