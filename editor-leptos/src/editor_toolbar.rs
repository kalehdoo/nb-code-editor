use leptos::prelude::*;
#[component]
pub fn EditorToolbar(
    dirty: ReadSignal<bool>,
    read_only: bool,
    saving: ReadSignal<bool>,
    on_save: Callback<()>,
    on_validate: Callback<()>,
) -> impl IntoView {
    view! { <div class="nb-toolbar"><button type="button" disabled=move || read_only || saving.get() || !dirty.get() on:click=move |_| on_save.run(())>"Save"</button><button type="button" on:click=move |_| on_validate.run(())>"Validate"</button><span role="status">{move || if saving.get() { "Saving…" } else if dirty.get() { "Unsaved changes" } else { "Saved" }}</span></div> }
}
