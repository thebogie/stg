//! Change or remove contest thumbnail on the details page (creator or admin).

use crate::api::contests::{
    delete_contest_image, read_contest_image_file, upload_contest_image,
};
use crate::components::contest::thumbnail::ContestThumbnail;
use web_sys::Url;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ContestThumbnailEditorProps {
    pub contest_id: String,
    pub image_url: Option<String>,
    #[prop_or_default]
    pub image_detail_url: Option<String>,
    pub can_edit: bool,
    pub on_image_url_change: Callback<(Option<String>, Option<String>)>,
}

#[function_component(ContestThumbnailEditor)]
pub fn contest_thumbnail_editor(props: &ContestThumbnailEditorProps) -> Html {
    let uploading = use_state(|| false);
    let error = use_state(|| None::<String>);
    let preview_url = use_state(|| None::<String>);

    let on_pick = {
        let contest_id = props.contest_id.clone();
        let uploading = uploading.clone();
        let error = error.clone();
        let preview_url = preview_url.clone();
        let on_image_url_change = props.on_image_url_change.clone();
        Callback::from(move |e: Event| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            let Some(file_list) = input.files() else {
                return;
            };
            if file_list.length() == 0 {
                return;
            }
            let file = file_list.get(0).unwrap();
            let contest_id = contest_id.clone();
            let uploading = uploading.clone();
            let error = error.clone();
            let preview_url = preview_url.clone();
            let on_image_url_change = on_image_url_change.clone();
            wasm_bindgen_futures::spawn_local(async move {
                uploading.set(true);
                error.set(None);

                match read_contest_image_file(file).await {
                    Ok((bytes, mime)) => {
                        let parts = js_sys::Array::new();
                        parts.push(&js_sys::Uint8Array::from(bytes.as_slice()).into());
                        let bag = web_sys::BlobPropertyBag::new();
                        bag.set_type(&mime);
                        if let Ok(blob) =
                            web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &bag)
                        {
                            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                preview_url.set(Some(url));
                            }
                        }

                        match upload_contest_image(&contest_id, bytes, &mime).await {
                            Ok(dto) => {
                                on_image_url_change.emit((dto.image_url, dto.image_detail_url));
                                if let Some(prev) = (*preview_url).clone() {
                                    let _ = Url::revoke_object_url(&prev);
                                }
                                preview_url.set(None);
                            }
                            Err(e) => {
                                error.set(Some(e));
                                if let Some(prev) = (*preview_url).clone() {
                                    let _ = Url::revoke_object_url(&prev);
                                }
                                preview_url.set(None);
                            }
                        }
                    }
                    Err(e) => error.set(Some(e)),
                }
                uploading.set(false);
            });
        })
    };

    let on_remove = {
        let contest_id = props.contest_id.clone();
        let uploading = uploading.clone();
        let error = error.clone();
        let preview_url = preview_url.clone();
        let on_image_url_change = props.on_image_url_change.clone();
        Callback::from(move |_| {
            if !gloo::dialogs::confirm("Remove this contest thumbnail?") {
                return;
            }
            let contest_id = contest_id.clone();
            let uploading = uploading.clone();
            let error = error.clone();
            let preview_url = preview_url.clone();
            let on_image_url_change = on_image_url_change.clone();
            wasm_bindgen_futures::spawn_local(async move {
                uploading.set(true);
                error.set(None);
                if let Some(prev) = (*preview_url).clone() {
                    let _ = Url::revoke_object_url(&prev);
                }
                preview_url.set(None);

                match delete_contest_image(&contest_id).await {
                    Ok(()) => on_image_url_change.emit((None, None)),
                    Err(e) => error.set(Some(e)),
                }
                uploading.set(false);
            });
        })
    };

    let display_url = (*preview_url).clone().or_else(|| props.image_url.clone());
    let has_image = display_url.is_some();

    html! {
        <div class="flex flex-col gap-2">
            if let Some(preview) = (*preview_url).clone() {
                <img
                    src={preview}
                    alt="Thumbnail preview"
                    class="w-20 h-20 rounded-lg object-cover border-2 border-white/30 shrink-0"
                />
            } else {
                <ContestThumbnail
                    image_url={props.image_url.clone()}
                    image_detail_url={props.image_detail_url.clone()}
                    class="w-20 h-20 rounded-lg border-2 border-white/30 shrink-0"
                    placeholder_class="w-20 h-20 rounded-lg bg-white/20 shrink-0 flex items-center justify-center text-2xl"
                    preview_on_hover={true}
                    expand_on_click={true}
                />
            }
            if props.can_edit {
                <div class="flex flex-wrap items-center gap-2 text-sm">
                    <label class="inline-flex items-center px-2 py-1 rounded bg-white/20 hover:bg-white/30 cursor-pointer text-white border border-white/30">
                        if *uploading {
                            {"Uploading…"}
                        } else {
                            {if has_image { "Change image" } else { "Add image" }}
                        }
                        <input
                            type="file"
                            accept="image/jpeg,image/png,image/webp"
                            class="hidden"
                            disabled={*uploading}
                            onchange={on_pick}
                        />
                    </label>
                    if props.image_url.is_some() && (*preview_url).is_none() {
                        <button
                            type="button"
                            class="px-2 py-1 rounded text-white/90 hover:text-white underline disabled:opacity-50"
                            disabled={*uploading}
                            onclick={on_remove}
                        >
                            {"Remove"}
                        </button>
                    }
                </div>
                if let Some(err) = (*error).clone() {
                    <p class="text-xs text-red-100 max-w-xs">{err}</p>
                }
            }
        </div>
    }
}
