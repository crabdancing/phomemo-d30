use std::{
    io::{self, Read},
    process::exit,
};

use show_image::{
    error::{CreateWindowError, InvalidWindowId, SetImageError},
    event::{VirtualKeyCode, WindowKeyboardInputEvent},
};
use snafu::{ResultExt, Snafu};

enum Accepted {
    Yes,
    No,
    Unknown,
}

#[derive(Debug, Snafu)]
enum CLIPreviewError {
    #[snafu(display("Failed to create window for preview"))]
    FailedToCreateWindow { source: CreateWindowError },
    #[snafu(display("Failed to set image for preview"))]
    FailedToSetImage { source: SetImageError },

    #[snafu(display("Invalid window ID"))]
    InvalidWindowID { source: InvalidWindowId },

    #[snafu(display("Failed to read image from STDIN"))]
    FailedToReadImageFromStdin { source: std::io::Error },
}

#[show_image::main]
fn main() -> Result<(), CLIPreviewError> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn,naga=off"),
    );

    let mut reader = io::stdin();

    let mut accepted = Accepted::Unknown;
    let mut buffer = Vec::new();
    reader
        .read_to_end(&mut buffer)
        .context(FailedToReadImageFromStdinSnafu)?;

    let preview_image = image::load_from_memory_with_format(&buffer, image::ImageFormat::Png)
        .expect("Failed to load");

    let var_name = "image";
    let window = show_image::create_window(var_name, Default::default())
        .context(FailedToCreateWindowSnafu)?;
    window
        .set_image("image-001", preview_image)
        .context(FailedToSetImageSnafu)?;
    'event_loop: for event in window.event_channel().context(InvalidWindowIDSnafu)? {
        match event {
            show_image::event::WindowEvent::CloseRequested(_) => {
                accepted = Accepted::Unknown;
                break 'event_loop;
            }
            show_image::event::WindowEvent::Destroyed(_) => {
                accepted = Accepted::Unknown;
                break 'event_loop;
            }
            // show_image::event::WindowEvent::FocusLost(_) => {
            //     accepted = Accepted::Unknown;
            //     break 'event_loop;
            // }
            show_image::event::WindowEvent::KeyboardInput(input) => match input {
                WindowKeyboardInputEvent { input, .. } => match input {
                    show_image::event::KeyboardInput { key_code, .. } => match key_code {
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
                    },
                },
            },
            _ => {}
        }
    }

    exit(match accepted {
        Accepted::Yes => 0,
        Accepted::No => 46,
        Accepted::Unknown => 47,
    });
}
