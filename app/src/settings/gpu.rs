use ai_terminal_ui::platform::GraphicsBackend;
use settings::macros::define_settings_group;
use settings::{SupportedPlatforms, SyncToCloud};

define_settings_group!(GPUSettings, settings: [
   prefer_low_power_gpu: PreferLowPowerGPU {
       type: bool,
       // Opt for the low power (integrated) GPU on Windows / Linux since discrete GPUs tend to be
        // more unstable.
       default: cfg!(any(target_os = "linux", target_os = "freebsd", windows)),
       supported_platforms: SupportedPlatforms::ALL,
       sync_to_cloud: SyncToCloud::Never,
       surface: settings::SettingSurfaces::GUI,
       private: false,
       toml_path: "system.prefer_low_power_gpu",
       description: "Whether to use the integrated (low-power) GPU instead of the discrete GPU.",
   },
   preferred_backend: PreferredGraphicsBackend {
       type: Option<GraphicsBackend>,
       default: None,
       supported_platforms: SupportedPlatforms::WINDOWS,
       sync_to_cloud: SyncToCloud::Never,
       surface: settings::SettingSurfaces::GUI,
       private: false,
       toml_path: "system.preferred_graphics_backend",
       description: "The preferred graphics backend on Windows.",
   },
]);
