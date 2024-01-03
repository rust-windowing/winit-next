use crate::application::Application;

pub trait TouchInputHandler: Application {
    fn touch_down(&mut self);

    fn touch_up(&mut self);
}
