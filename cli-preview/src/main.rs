use std::{
    io::{self, Read},
    process::exit,
};

use show_image::event::{VirtualKeyCode, WindowKeyboardInputEvent};
use snafu::{ResultExt, Whatever};

enum Accepted {
    Yes,
    No,
    Unknown,
}

#[show_image::main]
fn main() -> Result<(), Whatever> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn,naga=off"),
    );

    let mut reader = io::stdin();

    let mut accepted = Accepted::Unknown;
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer);

    // println!("Read bytes: {:?}", buffer);

    let preview_image = image::load_from_memory_with_format(&buffer, image::ImageFormat::Png)
        .expect("Failed to load");

    let window = show_image::create_window("image", Default::default())
        .with_whatever_context(|_| "Could not create window for preview")?;
    window
        .set_image("image-001", preview_image)
        .with_whatever_context(|_| "Could not set image")?;
    'event_loop: for event in window
        .event_channel()
        .with_whatever_context(|_| "Could not handle window channel")?
    {
        match event {
            // show_image::event::WindowEvent::RedrawRequested(_) => todo!(),
            // show_image::event::WindowEvent::Resized(_) => todo!(),
            // show_image::event::WindowEvent::Moved(_) => todo!(),
            show_image::event::WindowEvent::CloseRequested(_) => {
                accepted = Accepted::Unknown;
                break 'event_loop;
            }
            show_image::event::WindowEvent::Destroyed(_) => {
                accepted = Accepted::Unknown;
                break 'event_loop;
            }
            // show_image::event::WindowEvent::DroppedFile(_) => todo!(),
            // show_image::event::WindowEvent::HoveredFile(_) => todo!(),
            // show_image::event::WindowEvent::HoveredFileCancelled(_) => todo!(),
            // show_image::event::WindowEvent::FocusGained(_) => todo!(),
            show_image::event::WindowEvent::FocusLost(_) => {
                accepted = Accepted::Unknown;
                break 'event_loop;
            }
            show_image::event::WindowEvent::KeyboardInput(input) => match input {
                WindowKeyboardInputEvent { input, .. } => match input {
                    show_image::event::KeyboardInput { key_code, .. } => {
                        match key_code {
                            Some(show_image::event::VirtualKeyCode::Y) => {
                                accepted = Accepted::Yes;
                                break 'event_loop;
                            }
                            Some(show_image::event::VirtualKeyCode::N) => {
                                accepted = Accepted::No;
                                break 'event_loop;
                            }
                            Some(VirtualKeyCode::Q) => {
                                break 'event_loop;
                            }
                            Some(VirtualKeyCode::Escape) => {
                                break 'event_loop;
                            }
                            _ => {}
                        }
                        dbg!(input);
                    }
                },
            },
            // show_image::event::WindowEvent::TextInput(_) => todo!(),
            // show_image::event::WindowEvent::MouseEnter(_) => todo!(),
            // show_image::event::WindowEvent::MouseLeave(_) => todo!(),
            // show_image::event::WindowEvent::MouseMove(_) => todo!(),
            // show_image::event::WindowEvent::MouseButton(_) => todo!(),
            // show_image::event::WindowEvent::MouseWheel(_) => todo!(),
            // show_image::event::WindowEvent::AxisMotion(_) => todo!(),
            // show_image::event::WindowEvent::TouchpadPressure(_) => todo!(),
            // show_image::event::WindowEvent::Touch(_) => todo!(),
            // show_image::event::WindowEvent::ScaleFactorChanged(_) => todo!(),
            // show_image::event::WindowEvent::ThemeChanged(_) => todo!(),
            _ => {}
        }
    }

    exit(match accepted {
        Accepted::Yes => 0,
        Accepted::No => 46,
        Accepted::Unknown => 47,
    });
}
