use sctk::reexports::client::protocol::wl_output::WlOutput;
use sctk::reexports::client::Proxy;

use sctk::output::{OutputData, OutputHandler, OutputState};

use wayland_client::{Connection, QueueHandle};
use winit_core::application::Application;
use winit_core::dpi::{LogicalPosition, PhysicalPosition, PhysicalSize};
use winit_core::monitor::{Monitor as CoreMonitor, MonitorId};

use crate::event_loop::RuntimeState;

#[derive(Debug, PartialEq, Eq)]
pub struct Monitor {
    pub(crate) output: WlOutput,
}

impl Monitor {
    pub(crate) fn new(output: WlOutput) -> Self {
        Self { output }
    }
}

impl CoreMonitor for Monitor {
    fn id(&self) -> MonitorId {
        crate::make_mid(&self.output)
    }

    fn name(&self) -> Option<String> {
        let output_data = self.output.data::<OutputData>().unwrap();
        output_data.with_output_info(|info| info.name.clone())
    }

    fn size(&self) -> PhysicalSize<u32> {
        let output_data = self.output.data::<OutputData>().unwrap();
        let dimensions = output_data.with_output_info(|info| {
            info.modes.iter().find_map(|mode| mode.current.then_some(mode.dimensions))
        });

        match dimensions {
            Some((width, height)) => (width as u32, height as u32),
            _ => (0, 0),
        }
        .into()
    }

    fn position(&self) -> PhysicalPosition<i32> {
        let output_data = self.output.data::<OutputData>().unwrap();
        output_data.with_output_info(|info| {
            info.logical_position.map_or_else(
                || {
                    LogicalPosition::<i32>::from(info.location)
                        .to_physical(info.scale_factor as f64)
                },
                |logical_position| {
                    LogicalPosition::<i32>::from(logical_position)
                        .to_physical(info.scale_factor as f64)
                },
            )
        })
    }

    fn refresh_rate_millihertz(&self) -> Option<u32> {
        let output_data = self.output.data::<OutputData>().unwrap();
        output_data.with_output_info(|info| {
            info.modes.iter().find_map(|mode| mode.current.then_some(mode.refresh_rate as u32))
        })
    }

    fn scale_factor(&self) -> f64 {
        let output_data = self.output.data::<OutputData>().unwrap();
        output_data.scale_factor() as f64
    }
}

impl<T: Application + 'static> OutputHandler for RuntimeState<T> {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.winit.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.winit.monitors.push(Monitor::new(output));
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, updated: WlOutput) {
        // We dynamically load the output data from the proxy, thus we can
        // ignore this implementation.
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, removed: WlOutput) {
        self.winit.monitors.retain(|monitor| monitor.output != removed);
    }
}

sctk::delegate_output!(@<T: Application + 'static> RuntimeState<T>);
