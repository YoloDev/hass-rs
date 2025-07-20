# Changelog

## 0.1.0 (2025-07-20)


### Features

* add tracing ([#33](https://github.com/YoloDev/hass-rs/issues/33)) ([f833af1](https://github.com/YoloDev/hass-rs/commit/f833af1c61de8dd79ebfe66dd6488ab126312960))
* partial no-std support ([#49](https://github.com/YoloDev/hass-rs/issues/49)) ([8391c24](https://github.com/YoloDev/hass-rs/commit/8391c245dc2f524795cbc3505d1604ab7abc5184))

## [0.2.0](https://github.com/YoloDev/hass-rs/compare/hass-mqtt-discovery-v0.1.0...hass-mqtt-discovery-v0.2.0) (2022-11-27)


### ⚠ BREAKING CHANGES

* large API rewrite
* Changes type of the `Switch` field `command_topic`.
* Removes `Default` impl from `Switch` entities.
* Moves common fields to an `entity` field.

### Features

* add `availability` to `Sensor` entity ([ad26040](https://github.com/YoloDev/hass-rs/commit/ad26040f7a85a359f32b5b011508300af9da5be3))
* add `value_template` to Availability ([6b34e43](https://github.com/YoloDev/hass-rs/commit/6b34e434440f7a3666f97faa13739c5ac8808dde))
* add more discovery entities ([#17](https://github.com/YoloDev/hass-rs/issues/17)) ([a912eb7](https://github.com/YoloDev/hass-rs/commit/a912eb7b8ce80cb8ed64a15e49be48f7d6751a54))
* Common `Entity` struct ([#14](https://github.com/YoloDev/hass-rs/issues/14)) ([b800bbd](https://github.com/YoloDev/hass-rs/commit/b800bbdbf651f0790ca3c760a661b2831ebc3d02))
* Cover discovery entity ([#22](https://github.com/YoloDev/hass-rs/issues/22)) ([5829123](https://github.com/YoloDev/hass-rs/commit/5829123a78b2e482a83141fed2cb143137b40b72))
* DeviceTracker entity ([#18](https://github.com/YoloDev/hass-rs/issues/18)) ([0cbc757](https://github.com/YoloDev/hass-rs/commit/0cbc7572ec208671111ee69618286e3dc044b5ee))
* discovery document macro ([#19](https://github.com/YoloDev/hass-rs/issues/19)) ([1278754](https://github.com/YoloDev/hass-rs/commit/1278754bd5e559df1a4d012903ea65df9b25589b))
* fill in more `Sensor` discovery fields ([6ed9540](https://github.com/YoloDev/hass-rs/commit/6ed95401dbabc4742f422c7f16a49ddef798ab99))
* impl Default for Device ([b477b1f](https://github.com/YoloDev/hass-rs/commit/b477b1fbc7f9cbcedd2497c7f907d080f11475e4))
* impl traits for AvailabilityMode ([2e9f58d](https://github.com/YoloDev/hass-rs/commit/2e9f58d38e79c26a43f883d059f084d47db03de1))
* support more discovered Device fields ([5d5a310](https://github.com/YoloDev/hass-rs/commit/5d5a310f5d2eae156961fd5e27cef7a0c6259fe1))


### Bug Fixes

* change type of the `Switch` field `command_topic` ([dbd0fe5](https://github.com/YoloDev/hass-rs/commit/dbd0fe5958f6cf6fb329b572f77336762fc9c1c7))
* normalize device_class ([#23](https://github.com/YoloDev/hass-rs/issues/23)) ([40f02d9](https://github.com/YoloDev/hass-rs/commit/40f02d9d84b9bb9166956a933a670dac8e3970ac))
* remove `Default` impl from `Switch` entities ([dbd0fe5](https://github.com/YoloDev/hass-rs/commit/dbd0fe5958f6cf6fb329b572f77336762fc9c1c7))
* require `command_topic` ([#15](https://github.com/YoloDev/hass-rs/issues/15)) ([dbd0fe5](https://github.com/YoloDev/hass-rs/commit/dbd0fe5958f6cf6fb329b572f77336762fc9c1c7))


### Dependencies

* update rust crate semval to 0.5 ([#20](https://github.com/YoloDev/hass-rs/issues/20)) ([52b2d1a](https://github.com/YoloDev/hass-rs/commit/52b2d1aa82a0bfbc0622cd92a6d9ec9bfe16df0d))

## 0.1.0 (2021-11-16)


### Features

* rename proto and add entity-state ([28a6ce8](https://www.github.com/YoloDev/hass-rs/commit/28a6ce8fb36cf31b2f57d49d7a4ab31c867a33fd))
