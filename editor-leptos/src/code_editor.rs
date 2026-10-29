use crate::{
    editor_status::EditorStatus, editor_toolbar::EditorToolbar, line_numbers::LineNumbers,
    validation_panel::ValidationPanel,
};
use editor_core::{indent, newline, validate, Language, ValidationError};
use leptos::{html::Textarea, prelude::*};
use wasm_bindgen::JsCast;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    Auto,
}

/// `on_save` must validate and persist on the server, then acknowledge success or failure.
/// Content is initial-only; remount the component to open another document.
#[component]
pub fn CodeEditor(
    #[prop(into)] initial_content: String,
    language: Language,
    #[prop(into, default = "untitled".into())] file_name: String,
    #[prop(default = false)] read_only: bool,
    #[prop(default = Theme::Auto)] theme: Theme,
    #[prop(optional)] indentation_size: Option<usize>,
    #[prop(into, default = "320px".into())] height: String,
    on_save: Callback<(String, Callback<Result<(), String>>)>,
    #[prop(optional)] on_validation: Option<Callback<Vec<ValidationError>>>,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    // HTML textareas normalize line endings; keep the saved baseline consistent.
    let initial_content = initial_content.replace("\r\n", "\n").replace('\r', "\n");
    let area = NodeRef::<Textarea>::new();
    let baseline = StoredValue::new(initial_content.clone());
    let (dirty, set_dirty) = signal(false);
    let (saving, set_saving) = signal(false);
    let escape_tab = StoredValue::new(false);
    let (lines, set_lines) = signal(initial_content.bytes().filter(|b| *b == b'\n').count() + 1);
    let (characters, set_characters) = signal(initial_content.chars().count());
    let (scroll, set_scroll) = signal(0);
    let (errors, set_errors) = signal(Vec::<ValidationError>::new());
    let (validated, set_validated) = signal(false);
    let (save_error, set_save_error) = signal(String::new());
    let sync = Callback::new(move |text: String| {
        set_dirty.set(baseline.with_value(|b| b != &text));
        set_lines.set(text.bytes().filter(|b| *b == b'\n').count() + 1);
        set_characters.set(text.chars().count());
        set_validated.set(false);
        if let Some(cb) = on_change {
            cb.run(text);
        }
    });
    let check = Callback::new(move |()| {
        if let Some(el) = area.get() {
            let result = validate(&el.value(), language);
            if let Some(cb) = on_validation {
                cb.run(result.clone());
            }
            set_errors.set(result);
            set_validated.set(true);
        }
    });
    let save = Callback::new(move |()| {
        if read_only || saving.get_untracked() {
            return;
        }
        check.run(());
        if !errors.get_untracked().is_empty() {
            return;
        }
        if let Some(el) = area.get() {
            let text = el.value();
            let saved = text.clone();
            set_saving.set(true);
            let ack = Callback::new(move |result: Result<(), String>| {
                set_saving.set(false);
                match result {
                    Ok(()) => {
                        baseline.set_value(saved.clone());
                        set_dirty.set(area.get().is_some_and(|el| el.value() != saved));
                        set_save_error.set(String::new());
                    }
                    Err(message) => set_save_error.set(message),
                }
            });
            on_save.run((text, ack));
        }
    });
    let size = indentation_size
        .unwrap_or(language.default_indent())
        .clamp(1, 16);
    view! {
        <section class="nb-editor" data-theme=match theme { Theme::Light => "light", Theme::Dark => "dark", Theme::Auto => "auto" }>
            <header class="nb-header"><strong>{file_name.clone()}</strong><span>{language.label()}</span></header>
            <EditorToolbar dirty read_only saving on_save=save on_validate=check />
            <div class="nb-body" style:height=height>
                <LineNumbers lines scroll />
                <textarea node_ref=area aria-label=file_name aria-description="Tab indents; Shift+Tab outdents. Press Escape then Tab to move focus out of the editor." readonly=read_only spellcheck="false" autocapitalize="off" autocomplete="off" wrap="off"
                    on:input=move |_| { if let Some(el) = area.get() { sync.run(el.value()); } }
                    on:scroll=move |_| { if let Some(el) = area.get() { set_scroll.set(el.scroll_top()); } }
                    on:keydown=move |event: web_sys::KeyboardEvent| {
                        if read_only || event.is_composing() || event.ctrl_key() || event.meta_key() || event.alt_key() { return; }
                        if event.key() == "Escape" { escape_tab.set_value(true); return; }
                                                if event.key() == "Tab" && escape_tab.get_value() { escape_tab.set_value(false); return; }
                                                escape_tab.set_value(false);
                                                if event.key() != "Tab" && event.key() != "Enter" { return; }
                        let Some(el) = area.get() else { return; };
                        let start = el.selection_start().ok().flatten().unwrap_or(0);
                        let end = el.selection_end().ok().flatten().unwrap_or(start);
                        let edit = if event.key() == "Tab" { indent(&el.value(), start, end, size, event.shift_key()) } else { newline(&el.value(), start, end) };
                        if edit.start == edit.end && edit.replacement.is_empty() { event.prevent_default(); return; }
                                                // execCommand is deprecated, but unlike setRangeText it retains native undo
                        // transactions in supporting browsers. Do not replace with value assignment.
                        let Some(doc) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.dyn_into::<web_sys::HtmlDocument>().ok()) else { return; };
                        let _ = el.set_selection_range(edit.start, edit.end);
                        let command = if edit.replacement.is_empty() { "delete" } else { "insertText" };
                        if doc.exec_command_with_show_ui_and_value(command, false, &edit.replacement).unwrap_or(false) {
                            event.prevent_default();
                            let _ = el.set_selection_range(edit.selection_start, edit.selection_end);
                        } else {
                            let _ = el.set_selection_range(start, end);
                            // Unsupported browsers retain native Tab/Enter behavior and undo.
                        }
                    }
                >{initial_content}</textarea>
            </div>
            <ValidationPanel errors validated />
            <div role="alert" class="nb-validation">{save_error}</div>
            <EditorStatus language=language.label() lines characters read_only />
        </section>
    }
}
