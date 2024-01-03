# Exploring approaches to winit API 

**This is just a prototype showing _my_ vision for the future winit API.
The API itself is just rough idea on how it may look like**

The design is motivated by endless amount of issues where we need to model feedback
between the user and application and by the fact that GUI is generally single threaded.

The desired layout:

- `winit-core` - top-level traits not tied to any platform and ideally free from platform
  extensions.
- `winit` - glue around event loop creation in a cross platform way and managing extensions.
- `winit-wayland` - wayland backend implementing `winit-core`. Same should be done for other backends.
- `winit-examples` - examples for winit.

The example lives in the `winit-wayland`, but it'll be moved once the glue API is designed.
