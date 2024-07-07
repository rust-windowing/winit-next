use std::mem;

use raw_window_handle::{HandleError, HasWindowHandle, WaylandWindowHandle, WindowHandle};
use raw_window_handle_05::HasRawWindowHandle as HasRawWindowHandle05;

use sctk::shell::{xdg::{window::WindowConfigure, XdgSurface}, WaylandSurface};
use wayland_client::protocol::wl_surface::WlSurface;
use winit_core::{application::Application, window::Surface};

use crate::{popup::Popup, toplevel::Toplevel};



pub(crate) enum Window<T: Application + 'static> {
    Toplevel(Toplevel<T>),
    Popup(Popup<T>)
}

impl<T: Application + 'static> Window<T> {
    pub(crate) fn configured(&self) -> bool {
        match self {
            Window::Toplevel(toplevel) => toplevel.configured(),
            Window::Popup(popup) => popup.configured(),
        }
    }

    pub(crate) fn take_redraw(&mut self) -> bool {
        mem::take(match self {
            Window::Toplevel(toplevel) => &mut toplevel.redraw,
            Window::Popup(popup) => &mut popup.redraw,
        })
    }

    pub(crate) fn is_xdg_surface(&self) -> bool {
        match self {
            Window::Toplevel(_) => true,
            Window::Popup(_) => true,
            #[allow(unreachable_patterns)]
            _ => false
        }
    }
}

impl<T: Application + 'static> WaylandSurface for Window<T> {
    fn wl_surface(&self) -> &wayland_client::protocol::wl_surface::WlSurface {
        match self {
            Window::Toplevel(toplevel) => toplevel.window.wl_surface(),
            Window::Popup(popup) => popup.popup.wl_surface(),
        }
    }
}

impl<T: Application + 'static> XdgSurface for Window<T> {
    fn xdg_surface(&self) -> &wayland_protocols::xdg::shell::client::xdg_surface::XdgSurface {
        match self {
            Window::Toplevel(toplevel) => toplevel.window.xdg_surface(),
            Window::Popup(popup) => popup.popup.xdg_surface(),
            #[allow(unreachable_patterns)]
            _ => panic!("Not an XDG surface")
            
        }
    }
}

impl<T: Application + 'static> Surface for Window<T> {
    fn id(&self) -> winit_core::window::WindowId {
        match self {
            Window::Toplevel(toplevel) => toplevel.id(),
            Window::Popup(popup) => popup.id(),
        }
    }

    fn scale_factor(&self) -> f64 {
        match self {
            Window::Toplevel(toplevel) => toplevel.scale_factor(),
            Window::Popup(popup) => popup.scale_factor(),
        }
    }

    fn request_redraw(&mut self) {
        match self {
            Window::Toplevel(toplevel) => toplevel.request_redraw(),
            Window::Popup(popup) => popup.request_redraw(),
        }
    }

    fn inner_size(&self) -> winit_core::dpi::PhysicalSize<u32> {
        match self {
            Window::Toplevel(toplevel) => toplevel.inner_size(),
            Window::Popup(popup) => popup.inner_size(),
        }
    }

    fn role(&self) -> winit_core::window::WindowRole {
        match self {
            Window::Toplevel(toplevel) => toplevel.role(),
            Window::Popup(popup) => popup.role(),
        }
    }

    fn role_mut(&mut self) -> winit_core::window::WindowRoleMut {
        match self {
            Window::Toplevel(toplevel) => toplevel.role_mut(),
            Window::Popup(popup) => popup.role_mut(),
        }
    }
}

impl<T: Application + 'static> HasWindowHandle for Window<T> {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        match self {
            Window::Toplevel(toplevel) => toplevel.window_handle(),
            Window::Popup(popup) => popup.window_handle(),
        }
    }
}

unsafe impl<T: Application + 'static> HasRawWindowHandle05 for Window<T> {
    fn raw_window_handle(&self) -> raw_window_handle_05::RawWindowHandle {
        match self {
            Window::Toplevel(toplevel) => toplevel.raw_window_handle(),
            Window::Popup(popup) => popup.raw_window_handle(),
        }
    }
}