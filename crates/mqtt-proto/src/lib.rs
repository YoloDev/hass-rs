#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(provide_any, feature(provide_any))]

#[cfg(feature = "alloc")]
extern crate alloc;

// pub(crate) mod document;
pub(crate) mod list;
pub(crate) mod string;
pub(crate) mod validation;

pub mod availability;
pub mod device;
pub mod device_class;
pub mod device_tracker_source_type;
// pub mod entity;
pub mod entity_category;
pub mod icon;
pub mod name;
pub mod payload;
pub mod qos;
pub mod retain_handling;
pub mod state_class;
pub mod template;
pub mod topic;
pub mod unique_id;

#[doc(no_inline)]
pub use availability::Availability;
#[doc(no_inline)]
pub use device::Device;
#[doc(no_inline)]
pub use device_class::DeviceClass;
#[doc(no_inline)]
pub use device_tracker_source_type::DeviceTrackerSourceType;
// #[doc(no_inline)]
// pub use entity::{BinarySensor, Button, Cover, DeviceTracker, Light, Sensor, Switch};
#[doc(no_inline)]
pub use entity_category::EntityCategory;
#[doc(no_inline)]
pub use icon::Icon;
#[doc(no_inline)]
pub use name::Name;
#[doc(no_inline)]
pub use payload::Payload;
#[doc(no_inline)]
pub use qos::MqttQoS;
#[doc(no_inline)]
pub use retain_handling::MqttRetainHandling;
#[doc(no_inline)]
pub use state_class::StateClass;
#[doc(no_inline)]
pub use template::Template;
#[doc(no_inline)]
pub use topic::Topic;
#[doc(no_inline)]
pub use unique_id::UniqueId;

// #[doc(inline)]
// pub use document::Document;

#[doc(inline)]
pub use list::HassItems;
#[doc(inline)]
pub use string::HassStr;
#[doc(inline)]
pub use validation::ValidationError;
