use std::collections::HashMap;
use wasm_bindgen::prelude::*;

// We'll access the global flatpickrManager directly via web_sys

// Rust wrapper for flatpickr functionality
pub struct FlatpickrManager {
    instances: HashMap<String, JsValue>,
}

impl FlatpickrManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    // Initialize flatpickr on an element
    pub fn init(
        &mut self,
        element_id: &str,
        initial_value: Option<&str>,
        on_change: Option<JsValue>,
    ) -> Result<(), JsValue> {
        let _element = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(element_id)
            .ok_or_else(|| JsValue::from_str(&format!("Element '{}' not found", element_id)))?;

        // Create options object
        let options = js_sys::Object::new();
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("enableTime"),
            &JsValue::from_bool(true),
        )?;
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("time_24hr"),
            &JsValue::from_bool(true),
        )?;
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("minuteIncrement"),
            &JsValue::from_f64(5.0),
        )?;
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("dateFormat"),
            &JsValue::from_str("m/d/Y H:i"),
        )?;
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("allowInput"),
            &JsValue::from_bool(true),
        )?;
        js_sys::Reflect::set(
            &options,
            &JsValue::from_str("clickOpens"),
            &JsValue::from_bool(true),
        )?;

        if let Some(callback) = on_change {
            js_sys::Reflect::set(&options, &JsValue::from_str("onClose"), &callback)?;
        }

        // Call the global flatpickrManager.init function
        let manager = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &JsValue::from_str("flatpickrManager"),
        )?;
        let init_func = js_sys::Reflect::get(&manager, &JsValue::from_str("init"))?;
        let init_func: js_sys::Function = init_func.into();

        let result = init_func.call2(&manager, &JsValue::from_str(element_id), &options)?;

        if result.is_null() {
            return Err(JsValue::from_str("Failed to initialize flatpickr"));
        }

        self.instances.insert(element_id.to_string(), result);

        // Set initial value if provided
        if let Some(value) = initial_value {
            self.set_value(element_id, value)?;
        }

        Ok(())
    }

    // Set the value of a flatpickr instance
    pub fn set_value(&self, element_id: &str, value: &str) -> Result<(), JsValue> {
        let manager = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &JsValue::from_str("flatpickrManager"),
        )?;
        let set_value_func = js_sys::Reflect::get(&manager, &JsValue::from_str("setValue"))?;
        let set_value_func: js_sys::Function = set_value_func.into();

        set_value_func.call2(
            &manager,
            &JsValue::from_str(element_id),
            &JsValue::from_str(value),
        )?;
        Ok(())
    }

    // Get the value of a flatpickr instance
    pub fn get_value(&self, element_id: &str) -> Result<String, JsValue> {
        let manager = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &JsValue::from_str("flatpickrManager"),
        )?;
        let get_value_func = js_sys::Reflect::get(&manager, &JsValue::from_str("getValue"))?;
        let get_value_func: js_sys::Function = get_value_func.into();

        let result = get_value_func.call2(
            &manager,
            &JsValue::from_str(element_id),
            &JsValue::undefined(),
        )?;
        Ok(result.as_string().unwrap_or_default())
    }

    // Destroy a flatpickr instance
    pub fn destroy(&mut self, element_id: &str) -> Result<(), JsValue> {
        let manager = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &JsValue::from_str("flatpickrManager"),
        )?;
        let destroy_func = js_sys::Reflect::get(&manager, &JsValue::from_str("destroy"))?;
        let destroy_func: js_sys::Function = destroy_func.into();

        destroy_func.call2(
            &manager,
            &JsValue::from_str(element_id),
            &JsValue::undefined(),
        )?;
        self.instances.remove(element_id);
        Ok(())
    }

    // Destroy all instances
    pub fn destroy_all(&mut self) -> Result<(), JsValue> {
        let manager = js_sys::Reflect::get(
            &web_sys::window().unwrap(),
            &JsValue::from_str("flatpickrManager"),
        )?;
        let destroy_all_func = js_sys::Reflect::get(&manager, &JsValue::from_str("destroyAll"))?;
        let destroy_all_func: js_sys::Function = destroy_all_func.into();

        destroy_all_func.call1(&manager, &JsValue::undefined())?;
        self.instances.clear();
        Ok(())
    }
}

// Stateless helpers that call the global JS manager directly
pub fn fp_init(
    element_id: &str,
    initial_value: Option<&str>,
    on_change: Option<JsValue>,
) -> Result<(), JsValue> {
    // Create options object
    let options = js_sys::Object::new();
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("enableTime"),
        &JsValue::from_bool(true),
    )?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("time_24hr"),
        &JsValue::from_bool(true),
    )?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("minuteIncrement"),
        &JsValue::from_f64(5.0),
    )?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("dateFormat"),
        &JsValue::from_str("m/d/Y H:i"),
    )?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("allowInput"),
        &JsValue::from_bool(true),
    )?;
    js_sys::Reflect::set(
        &options,
        &JsValue::from_str("clickOpens"),
        &JsValue::from_bool(true),
    )?;
    if let Some(cb) = on_change {
        js_sys::Reflect::set(&options, &JsValue::from_str("onClose"), &cb)?;
    }

    let manager = js_sys::Reflect::get(
        &web_sys::window().unwrap(),
        &JsValue::from_str("flatpickrManager"),
    )?;
    let init_func = js_sys::Reflect::get(&manager, &JsValue::from_str("init"))?;
    let init_func: js_sys::Function = init_func.into();
    let _ = init_func.call2(&manager, &JsValue::from_str(element_id), &options)?;

    if let Some(value) = initial_value {
        fp_set_value(element_id, value)?;
    }
    Ok(())
}

pub fn fp_set_value(element_id: &str, value: &str) -> Result<(), JsValue> {
    let manager = js_sys::Reflect::get(
        &web_sys::window().unwrap(),
        &JsValue::from_str("flatpickrManager"),
    )?;
    let set_value_func = js_sys::Reflect::get(&manager, &JsValue::from_str("setValue"))?;
    let set_value_func: js_sys::Function = set_value_func.into();
    let _ = set_value_func.call2(
        &manager,
        &JsValue::from_str(element_id),
        &JsValue::from_str(value),
    )?;
    Ok(())
}

pub fn fp_destroy_all() -> Result<(), JsValue> {
    let manager = js_sys::Reflect::get(
        &web_sys::window().unwrap(),
        &JsValue::from_str("flatpickrManager"),
    )?;
    let destroy_all_func = js_sys::Reflect::get(&manager, &JsValue::from_str("destroyAll"))?;
    let destroy_all_func: js_sys::Function = destroy_all_func.into();
    let _ = destroy_all_func.call1(&manager, &JsValue::undefined())?;
    Ok(())
}

impl Drop for FlatpickrManager {
    fn drop(&mut self) {
        let _ = self.destroy_all();
    }
}
