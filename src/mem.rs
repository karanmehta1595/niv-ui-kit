//! Mem: ek value ko frame-dar-frame yaad rakhna (egui ke `ctx.memory` / `data` ki jagah).
//!
//! ```ignore
//! let mut tab = Mem::<usize>::new(ui, "tab");   // pehli baar Default (0)
//! if clicked { tab.set(2); }
//! let t = tab.get();
//! Mem::with(ui, "zoom", 1.0_f32).update(|z| *z *= 2.0); // shuru ki value ke saath
//! ```
//! Naam (`"tab"`) is `ui` ke andar unique hona chahiye; alag `ui`/window mein wahi naam alag value hai.
//! Value `Clone + Send + Sync + 'static` honi chahiye. App band hone par ye nahi bachti.

use egui::{Id, Ui};

/// `Mem::<T>::new(ui, "naam")`.
pub struct Mem<T> {
    ctx: egui::Context,
    id: Id,
    value: T,
}

impl<T: Clone + Default + Send + Sync + 'static> Mem<T> {
    /// Naam ki value, na ho to `T::default()`.
    pub fn new(ui: &Ui, name: &str) -> Self {
        Self::with(ui, name, T::default())
    }
}

impl<T: Clone + Send + Sync + 'static> Mem<T> {
    /// Naam ki value, na ho to `initial`.
    pub fn with(ui: &Ui, name: &str, initial: T) -> Self {
        let id = ui.scope_id().with(("niv_mem", name));
        let value = ui.ctx().data_mut(|d| d.get_temp::<T>(id)).unwrap_or(initial);
        Mem { ctx: ui.ctx().clone(), id, value }
    }

    /// Abhi ki value.
    pub fn get(&self) -> T {
        self.value.clone()
    }

    /// Nayi value rakho.
    pub fn set(&mut self, value: T) {
        self.value = value.clone();
        self.ctx.data_mut(|d| d.insert_temp(self.id, value));
    }

    /// Value ko badlo.
    pub fn update(&mut self, change: impl FnOnce(&mut T)) {
        let mut v = self.value.clone();
        change(&mut v);
        self.set(v);
    }
}
