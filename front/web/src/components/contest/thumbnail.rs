//! Authenticated contest thumbnail (GET requires Bearer token; served as WebP).

use crate::api::api_url;
use crate::api::utils::authenticated_get;
use web_sys::Url;
use yew::prelude::*;

async fn fetch_image_object_url(image_path: &str) -> Result<String, String> {
    let url = if image_path.starts_with("http://") || image_path.starts_with("https://") {
        image_path.to_string()
    } else {
        api_url(image_path)
    };
    let resp = authenticated_get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let bytes = resp.binary().await.map_err(|e| e.to_string())?;
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(bytes.as_slice()).into());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)
        .map_err(|_| "Failed to create blob")?;
    Url::create_object_url_with_blob(&blob).map_err(|_| "Failed to create object URL".to_string())
}

#[derive(Properties, PartialEq)]
pub struct ContestThumbnailProps {
    pub image_url: Option<String>,
    #[prop_or("w-10 h-10 rounded object-cover shrink-0".into())]
    pub class: AttrValue,
    #[prop_or("w-10 h-10 rounded bg-gray-200 shrink-0 flex items-center justify-center text-gray-400".into())]
    pub placeholder_class: AttrValue,
}

#[function_component(ContestThumbnail)]
pub fn contest_thumbnail(props: &ContestThumbnailProps) -> Html {
    let blob_url = use_state(|| None::<String>);
    let image_url = props.image_url.clone();

    {
        let blob_url = blob_url.clone();
        let image_url = image_url.clone();
        use_effect_with(image_url, move |url_opt| {
            let blob_url = blob_url.clone();
            let cancelled = std::rc::Rc::new(std::cell::Cell::new(false));

            if url_opt.is_none() {
                if let Some(prev) = (*blob_url).clone() {
                    let _ = Url::revoke_object_url(&prev);
                }
                blob_url.set(None);
                return Box::new(|| {}) as Box<dyn FnOnce()>;
            }

            let url = url_opt.clone().unwrap();
            let cancelled_fetch = cancelled.clone();
            let blob_url_fetch = blob_url.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_image_object_url(&url).await {
                    Ok(object_url) if !cancelled_fetch.get() => {
                        blob_url_fetch.set(Some(object_url));
                    }
                    _ if !cancelled_fetch.get() => {
                        blob_url_fetch.set(None);
                    }
                    _ => {}
                }
            });

            let cancelled_cleanup = cancelled.clone();
            let blob_url_cleanup = blob_url.clone();
            Box::new(move || {
                cancelled_cleanup.set(true);
                if let Some(prev) = (*blob_url_cleanup).clone() {
                    let _ = Url::revoke_object_url(&prev);
                }
            })
        });
    }

    if let Some(src) = (*blob_url).clone() {
        html! {
            <img src={src} alt="" class={props.class.clone()} />
        }
    } else if props.image_url.is_some() {
        html! {
            <div class={props.placeholder_class.clone()} aria-hidden="true">
                <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-500"></span>
            </div>
        }
    } else {
        html! {
            <div class={props.placeholder_class.clone()} aria-hidden="true">
                <span class="text-lg">{"🎯"}</span>
            </div>
        }
    }
}
