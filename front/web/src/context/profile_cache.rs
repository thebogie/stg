//! In-memory cache for profile data so navigating back to the profile page doesn't refetch.
//! Cache is keyed by profile param (e.g. "me" or player id) and has a TTL.

use shared::dto::analytics::ProfileBundleDto;
use std::collections::HashMap;
use yew::prelude::*;

/// TTL: treat cache as fresh for 5 minutes.
const CACHE_TTL_MS: f64 = 5.0 * 60.0 * 100.0;

/// One cache entry: bundle + when it was stored (ms since epoch, from `js_sys::Date::now()`).
#[derive(Clone, Debug)]
pub struct CachedProfileEntry {
    pub bundle: ProfileBundleDto,
    pub fetched_at_ms: f64,
}

impl CachedProfileEntry {
    pub fn is_fresh(&self, now_ms: f64) -> bool {
        now_ms - self.fetched_at_ms < CACHE_TTL_MS
    }
}

/// Context value: read/write cache keyed by profile param (e.g. "me", or encoded player id).
#[derive(Clone)]
pub struct ProfileCacheContextValue {
    pub cache: UseStateHandle<HashMap<String, CachedProfileEntry>>,
}

impl PartialEq for ProfileCacheContextValue {
    /// Same provider, same handle; Yew requires PartialEq for ContextProvider/use_effect_with.
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[derive(Properties, PartialEq)]
pub struct ProfileCacheProviderProps {
    pub children: Children,
}

#[function_component(ProfileCacheProvider)]
pub fn profile_cache_provider(props: &ProfileCacheProviderProps) -> Html {
    let cache = use_state(|| HashMap::<String, CachedProfileEntry>::new());
    let value = ProfileCacheContextValue {
        cache: cache.clone(),
    };
    html! {
        <ContextProvider<ProfileCacheContextValue> context={value}>
            {props.children.clone()}
        </ContextProvider<ProfileCacheContextValue>>
    }
}
