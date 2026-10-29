use leptos::prelude::*;
#[component]
pub fn LineNumbers(lines: ReadSignal<usize>, scroll: ReadSignal<i32>) -> impl IntoView {
    view! { <div class="nb-gutter" aria-hidden="true"><div style=move || format!("transform:translateY(-{}px)", scroll.get())>{move || (1..=lines.get()).map(|n| n.to_string()).collect::<Vec<_>>().join("\n")}</div></div> }
}
