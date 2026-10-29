use leptos::prelude::*;
#[component]
pub fn EditorStatus(
    language: &'static str,
    lines: ReadSignal<usize>,
    characters: ReadSignal<usize>,
    read_only: bool,
) -> impl IntoView {
    view! { <footer class="nb-status"><span>{language}</span><span>{move || format!("{} lines · {} characters", lines.get(), characters.get())}</span><span>{if read_only { "Read only" } else { "" }}</span></footer> }
}
