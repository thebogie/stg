// Flatpickr integration for stable date/time inputs
// This provides a stable interface between Rust WASM and flatpickr

class FlatpickrManager {
    constructor() {
        this.instances = new Map();
    }

    // Initialize flatpickr on an element
    init(elementId, options = {}) {
        console.log(`Attempting to initialize flatpickr on element: ${elementId}`);
        const element = document.getElementById(elementId);
        if (!element) {
            console.error(`Element with id '${elementId}' not found`);
            return null;
        }
        
        if (typeof flatpickr === 'undefined') {
            console.error('flatpickr library not loaded!');
            return null;
        }
        
        console.log('flatpickr library is available');

        // Default options for datetime-local behavior
        const defaultOptions = {
            enableTime: true,
            time_24hr: true,
            minuteIncrement: 5,
            dateFormat: "m/d/Y H:i",  // User-friendly format: 09/01/2025 15:45
            allowInput: true,
            clickOpens: true,
            onChange: (selectedDates, dateStr, instance) => {
                console.log(`Flatpickr onChange - selectedDates:`, selectedDates, `dateStr:`, dateStr);
                // Call the Rust callback if provided
                if (options.onChange) {
                    options.onChange(selectedDates, dateStr, instance);
                }
            },
            onClose: (selectedDates, dateStr, instance) => {
                console.log(`Flatpickr onClose - selectedDates:`, selectedDates, `dateStr:`, dateStr);
                // Call the Rust callback if provided
                if (options.onClose) {
                    options.onClose(selectedDates, dateStr, instance);
                }
            }
        };

        const finalOptions = { ...defaultOptions, ...options };
        
        // Destroy existing instance if it exists
        if (this.instances.has(elementId)) {
            this.destroy(elementId);
        }

        try {
            const instance = flatpickr(element, finalOptions);
            this.instances.set(elementId, instance);
            return instance;
        } catch (error) {
            console.error(`Failed to initialize flatpickr on ${elementId}:`, error);
            return null;
        }
    }

    // Set the value of a flatpickr instance
    setValue(elementId, value) {
        const instance = this.instances.get(elementId);
        if (instance) {
            instance.setDate(value, false); // false = don't trigger onChange
        }
    }

    // Get the value of a flatpickr instance
    getValue(elementId) {
        const instance = this.instances.get(elementId);
        if (instance) {
            return instance.input.value;
        }
        return null;
    }

    // Destroy a flatpickr instance
    destroy(elementId) {
        const instance = this.instances.get(elementId);
        if (instance) {
            instance.destroy();
            this.instances.delete(elementId);
        }
    }

    // Destroy all instances
    destroyAll() {
        for (const [elementId, instance] of this.instances) {
            instance.destroy();
        }
        this.instances.clear();
    }
}

// Global instance
window.flatpickrManager = new FlatpickrManager();

// Export for use in Rust
window.FlatpickrManager = FlatpickrManager;
