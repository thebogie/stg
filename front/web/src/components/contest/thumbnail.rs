//! Authenticated contest thumbnail (GET requires Bearer token; served as WebP).
//!
//! List UI uses the thumb URL (~160px). Hover preview and lightbox fetch the detail URL (~512px)
//! when available (lazy). Defaults to `object-contain` for group photos.

use crate::api::api_url;
use crate::api::utils::authenticated_get;
use gloo::events::EventListener;
use gloo::utils::document;
use wasm_bindgen::JsCast;
use web_sys::{MouseEvent, Url};
use yew::html::create_portal;
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

#[derive(Clone, Copy, PartialEq)]
pub enum ThumbnailFit {
    Contain,
    Cover,
}

impl ThumbnailFit {
    fn tailwind(self) -> &'static str {
        match self {
            ThumbnailFit::Contain => "object-contain",
            ThumbnailFit::Cover => "object-cover",
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct ContestThumbnailProps {
    pub image_url: Option<String>,
    /// Larger variant for hover/lightbox; falls back to `image_url` if unset.
    #[prop_or_default]
    pub image_detail_url: Option<String>,
    #[prop_or("w-10 h-10 rounded shrink-0 bg-gray-100".into())]
    pub class: AttrValue,
    #[prop_or("w-10 h-10 rounded bg-gray-200 shrink-0 flex items-center justify-center text-gray-400".into())]
    pub placeholder_class: AttrValue,
    #[prop_or(ThumbnailFit::Contain)]
    pub fit: ThumbnailFit,
    #[prop_or(true)]
    pub preview_on_hover: bool,
    #[prop_or(true)]
    pub expand_on_click: bool,
    #[prop_or("Contest photo".into())]
    pub alt: AttrValue,
}

#[function_component(ContestThumbnail)]
pub fn contest_thumbnail(props: &ContestThumbnailProps) -> Html {
    let thumb_blob = use_state(|| None::<String>);
    let detail_blob = use_state(|| None::<String>);
    let image_url = props.image_url.clone();
    let image_detail_url = props
        .image_detail_url
        .clone()
        .or_else(|| props.image_url.clone());
    let hover_preview = use_state(|| false);
    let lightbox_open = use_state(|| false);

    {
        let thumb_blob = thumb_blob.clone();
        let image_url = image_url.clone();
        use_effect_with(image_url, move |url_opt| {
            let thumb_blob = thumb_blob.clone();
            let cancelled = std::rc::Rc::new(std::cell::Cell::new(false));

            if url_opt.is_none() {
                if let Some(prev) = (*thumb_blob).clone() {
                    let _ = Url::revoke_object_url(&prev);
                }
                thumb_blob.set(None);
                return Box::new(|| {}) as Box<dyn FnOnce()>;
            }

            let url = url_opt.clone().unwrap();
            let cancelled_fetch = cancelled.clone();
            let thumb_blob_fetch = thumb_blob.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_image_object_url(&url).await {
                    Ok(object_url) if !cancelled_fetch.get() => {
                        thumb_blob_fetch.set(Some(object_url));
                    }
                    _ if !cancelled_fetch.get() => {
                        thumb_blob_fetch.set(None);
                    }
                    _ => {}
                }
            });

            let cancelled_cleanup = cancelled.clone();
            let thumb_blob_cleanup = thumb_blob.clone();
            Box::new(move || {
                cancelled_cleanup.set(true);
                if let Some(prev) = (*thumb_blob_cleanup).clone() {
                    let _ = Url::revoke_object_url(&prev);
                }
            })
        });
    }

    let load_detail = {
        let detail_blob = detail_blob.clone();
        let image_detail_url = image_detail_url.clone();
        let thumb_blob = thumb_blob.clone();
        Callback::from(move |_| {
            if (*detail_blob).is_some() {
                return;
            }
            let Some(detail_path) = image_detail_url.clone() else {
                return;
            };
            let detail_blob = detail_blob.clone();
            let thumb_blob = thumb_blob.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_image_object_url(&detail_path).await {
                    Ok(url) => detail_blob.set(Some(url)),
                    Err(_) => {
                        if let Some(fallback) = (*thumb_blob).clone() {
                            detail_blob.set(Some(fallback));
                        }
                    }
                }
            });
        })
    };

    {
        let lightbox_open = lightbox_open.clone();
        use_effect_with(*lightbox_open, move |open| {
            if !*open {
                return Box::new(|| {}) as Box<dyn FnOnce()>;
            }
            let lightbox_open = lightbox_open.clone();
            let listener = EventListener::new(&web_sys::window().unwrap(), "keydown", move |event| {
                if let Some(e) = event.dyn_ref::<web_sys::KeyboardEvent>() {
                    if e.key() == "Escape" {
                        lightbox_open.set(false);
                    }
                }
            });
            Box::new(move || drop(listener)) as Box<dyn FnOnce()>
        });
    }

    let fit_class = props.fit.tailwind();
    let mut img_class = format!("{} {}", props.class, fit_class);
    if props.expand_on_click {
        img_class.push_str(" cursor-zoom-in");
    }

    let large_src = (*detail_blob)
        .clone()
        .or_else(|| (*thumb_blob).clone());

    let on_thumb_click = {
        let expand = props.expand_on_click;
        let lightbox_open = lightbox_open.clone();
        let thumb_blob = thumb_blob.clone();
        let load_detail = load_detail.clone();
        Callback::from(move |e: MouseEvent| {
            if expand && (*thumb_blob).is_some() {
                e.stop_propagation();
                load_detail.emit(());
                lightbox_open.set(true);
            }
        })
    };

    let on_enter = {
        let preview_on_hover = props.preview_on_hover;
        let hover_preview = hover_preview.clone();
        let thumb_blob = thumb_blob.clone();
        let load_detail = load_detail.clone();
        Callback::from(move |_: MouseEvent| {
            if preview_on_hover && (*thumb_blob).is_some() {
                load_detail.emit(());
                hover_preview.set(true);
            }
        })
    };

    let on_leave = {
        let hover_preview = hover_preview.clone();
        Callback::from(move |_: MouseEvent| {
            hover_preview.set(false);
        })
    };

    let close_lightbox = {
        let lightbox_open = lightbox_open.clone();
        Callback::from(move |e: MouseEvent| {
            e.stop_propagation();
            lightbox_open.set(false);
        })
    };

    let thumb_body = if let Some(src) = (*thumb_blob).clone() {
        let preview_src = large_src.clone().unwrap_or(src.clone());
        html! {
            <div
                class="relative inline-block"
                onmouseenter={on_enter}
                onmouseleave={on_leave}
            >
                <img
                    src={src}
                    alt={props.alt.clone()}
                    class={img_class.clone()}
                    onclick={on_thumb_click}
                />
                if *hover_preview && props.preview_on_hover {
                    <div
                        class="hidden md:block absolute z-50 left-full ml-2 top-1/2 -translate-y-1/2 pointer-events-none"
                        aria-hidden="true"
                    >
                        <img
                            src={preview_src}
                            alt=""
                            class="max-w-[min(28rem,85vw)] max-h-[min(32rem,85vh)] object-contain rounded-lg shadow-xl border border-gray-200 bg-white p-1"
                        />
                    </div>
                }
                if props.expand_on_click {
                    <span class="sr-only">{"Click for larger view"}</span>
                }
            </div>
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
    };

    let lightbox = if *lightbox_open {
        large_src.map(|src| {
            html! {
                <div
                    class="fixed inset-0 z-[200] flex items-center justify-center bg-black/80 p-4"
                    onclick={close_lightbox.clone()}
                    role="dialog"
                    aria-modal="true"
                    aria-label="Contest photo"
                >
                    <button
                        type="button"
                        class="absolute top-4 right-4 text-white/90 hover:text-white text-3xl leading-none px-2"
                        onclick={close_lightbox.clone()}
                        aria-label="Close"
                    >
                        {"×"}
                    </button>
                    <img
                        src={src}
                        alt={props.alt.clone()}
                        class="max-w-full max-h-[85vh] object-contain rounded-lg shadow-2xl"
                        onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}
                    />
                </div>
            }
        })
    } else {
        None
    };

    html! {
        <>
            {thumb_body}
            if let Some(lightbox) = lightbox {
                if let Some(body) = document().body() {
                    {create_portal(lightbox, body.into())}
                }
            }
        </>
    }
}
