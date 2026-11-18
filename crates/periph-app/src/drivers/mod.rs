//! Driver registry. To add a device: write `drivers/<id>.rs` implementing
//! `periph_core::Driver`, add it below, and create the frontend module `app/src/devices/<id>/`.
//! The `id` returned by `Driver::info()` must match the frontend folder name.

mod via;
mod vt2;

use periph_core::Driver;

pub fn all() -> Vec<Box<dyn Driver>> {
    vec![
        Box::new(vt2::Vt2Driver),
        Box::new(via::ViaDriver),
        // Box::new(mydevice::MyDeviceDriver),
    ]
}
