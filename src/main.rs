mod capture;

use image::ImageReader;

use stream_controller_rs::{
    image_to_rgb565, Button, ButtonPressEvent, ConfirmFrameBufferInEvent, ControlInterface, Event,
    Message, MessageType, PressDirection, StreamControllerX,
};
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;

// TODO: Add a way to enable/disable certain events (especially ConfirmFrameBufferIn)

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Call SetProcessDPIAware to ensure correct DPI handling
    // Note: The SetProcessDPIAware function must be called before any UI-related code runs.
    unsafe {
        let _ = SetProcessDPIAware();
    }
    let test_image = image_to_rgb565(
        &ImageReader::open("C:/Project/Workspace/Rust/stream-controller-rs/test.png")?
            .decode()
            .unwrap(),
    );
    let test_button_image = image_to_rgb565(
        &ImageReader::open("C:/Project/Workspace/Rust/stream-controller-rs/accurate_button.png")?
            .decode()
            .unwrap(),
    );
    let stream_controller = StreamControllerX::new();
    let control_interface = stream_controller.control_interface();
    let mut my_app = MyApp {
        test_image,
        test_button_image,
        current_brightness: 10,
    };
    stream_controller
        .run(async move { my_app.event_handler(control_interface).await })
        .await
}

pub struct MyApp {
    test_image: Vec<u8>,
    test_button_image: Vec<u8>,
    current_brightness: u8,
}

impl MyApp {
    pub async fn event_handler(
        &mut self,
        control_interface: ControlInterface,
    ) -> std::io::Result<()> {
        let mut rx_event = control_interface.tx_event.subscribe();
        loop {
            tokio::select! {
                _ = control_interface.shutdown_token.cancelled() => return Ok(()),
                _ = capture::stream_screenshot(&control_interface) => {}
                next_event = rx_event.recv() =>
                {
                    if let Err(err) = next_event
                    {
                        eprintln!("{err:?}");
                        continue;
                    }
                    match next_event.unwrap()
                    {
                        Event::ButtonPress(ButtonPressEvent
                        {
                            tx_id,
                            button,
                            dir: PressDirection::Down,
                        }) =>
                        {
                            // println!("Button down: {tx_id:?}:{button:?}:{:?}", PressDirection::Down);
                            if button == Button::B00 {
                                self.current_brightness = self.current_brightness.saturating_sub(1);
                                let brightness_msg = Message
                                {
                                    mtype: MessageType::SetBrightness(self.current_brightness),
                                    tx_id: tx_id + 1
                                };
                                let tx_pending_send = control_interface.tx_pending_send.clone();
                                tokio::spawn(async move {
                                    tx_pending_send.send(brightness_msg).await.unwrap();
                                });
                            } else if button == Button::B40 {
                                self.current_brightness = (self.current_brightness + 1).min(10);
                                let brightness_msg = Message
                                {
                                    mtype: MessageType::SetBrightness(self.current_brightness),
                                    tx_id: tx_id + 1
                                };
                                let tx_pending_send = control_interface.tx_pending_send.clone();
                                tokio::spawn(async move {
                                    tx_pending_send.send(brightness_msg).await.unwrap();
                                });
                            }
                            let msg = Message
                            {
                                mtype: MessageType::DrawScreen(self.test_image.clone()),
                                tx_id: tx_id + 1
                            };
                            let button_msg = Message
                            {
                                mtype: MessageType::DrawButton(button, self.test_button_image.clone()),
                                tx_id: tx_id + 2
                            };

                            let tx_pending_send = control_interface.tx_pending_send.clone();
                            tokio::spawn(async move {
                                tx_pending_send.send(msg).await.unwrap();
                                tx_pending_send.send(button_msg).await.unwrap();
                            });

                        },
                        Event::ButtonPress(ButtonPressEvent
                        {
                            tx_id: _,
                            button: _,
                            dir: PressDirection::Up,
                        }) =>
                        {
                            // Nothing to do here yet
                        },
                        Event::ConfirmFrameBufferIn(ConfirmFrameBufferInEvent{tx_id: _}) =>
                        {
                            // Note: These events are disabled as they are too noisy at high frame rates
                        }
                        event =>
                        {
                            println!("Unhandled event: {event:?}");
                        }
                    }
                }
            };
        }
    }
}
