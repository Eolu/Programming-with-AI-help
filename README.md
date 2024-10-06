# What is this?

I made this repo to show that using AI to aid in development is pretty cool, and already quite useful. The code here is not amazing, and not what I'd call production-ready... however it was made over the course of a couple hours in a single night. Most code was written by AI (specifically openai-o1-preview). 

## The goal

I had a simple API, and I wanted o1-preview to take that and create a plugin that acted a bit like the windows "magnifier" tool: just show the screen on the buttons. The result, after 6 prompts and a very small amount of manual fixing (small errors made by o1 like forgetting unwraps, and some very simple integration into my API that I didn't bother getting into in the prompts), I ended up with something that just worked:

<img src="example.gif" alt="preview-image" width="230"/>

## Some background

I used similar techniques to develop the core interfaces to this device. There were a lot more prompts involved, a lot more "manual" coding, and I haven't prepared or shared them (or published the core interface at all at this point). The device is a `Razer Strem Controller X`, a Razer-branded [Loupedeck](https://loupedeck.com/)-made device. Really, it's just 15 buttons arranged in a grid with LED screens and a websocket-based serial interface. The proprietary software for it is rather opinionated and all C#/.net-based. I wanted to create something more low-level. I found some existing [javascript](https://github.com/foxxyz/loupedeck) and [rust](https://github.com/adierkens/loupedeck-rs)-based attempts at this, but I was really looking to make something even simpler. I did that and then looked to play around with AI to make plugins for it. This is where this repo comes in!

PS: It's all mostly hard-coded to be specific to this device, but if I get things to a nicer/more-generalized state I may release these APIs at some later date.

## Why am I sharing this?

I still see a lot of people who think AI isn't a developer tool worth taking seriously. I think that's already not true - and is going to be increasingly untrue with time. I don't think AI is currently on a path to replace the need for engineers - but it's certainly on the path where if you're an engineer that refuses to use it - you're going to be left in the dust. The task I made it do here wasn't tremendously hard, especially as I provided the exact interface it needed to adhere to. But the time it would've taken me to research all the APIs needed, figure out how to use them, and implement this bare-minimum thing would've been much longer. The time from making my first prompt and seeing something function was measurable in a single-digit number of minutes. Fixing all of the bugs and making it do what I originally wanted took only 5 additional prompts. 

Now for the good stuff: below I've pasted my entire transcript and conversation with o1-preview. I've never used the windows crate or even understood how it worked - I just naively pasted in the stuff it told me. I wouldn't do this in a production environment, but I would certainly do it in a rapid-prototyping environment. And this was... really, REALLY rapid.

### tl;dr on how this went:

- The first prompt gave the core ask for functionality. This gave me something with a few syntax errors, but once I fixed those it worked... almost. There was a bug in the display the ended up being a lack of account for DPI-scaling, and also a lack of a mouse-pointer on-screen.
- Prompt 2 asked to show the mouse-pointer. This worked but the pointer was not centered properly.
- Prompt 3 and 4 asked to fix the problem with the display. Neither of them solved it, but they interestingly provided some fixes for problems I hadn't yet realized were problems.
- Prompt 5 gave the description of the problem in more detail (I gacve specific information about the way in which the screen was incorrectly rendering). At this point openai-o1 correctly guessed that the issue had to do with DPI-scaling, and gave the solution.
- Prompt 6 asked to fix the off-centered mouse-pointer. The correct solution was given right off the bat.

---
# Transcript of conversation with o1-preview

### Prompt 1

I have a Rust program on Windows that interacts with a device with a 480x270 pixel rgb565 LED screen. This program uses a tokio mpsc channel to send messages that get sent to the device. I would like to send a 60fps stream of screenshots to be displayed on the device. These screenshots should display a 480x270 area centered around the mouse-pointer. They should be able to capture anything on the Windows display within that location.

I've included a few key pieces of code already implemented within the program that may be helpful:

```rust
/// Helper function to convert an image to an rgb565 image, used by this device
pub fn image_to_rgb565(image: &image::DynamicImage) -> Vec<u8> {
    // implementation left out (uses rayon for efficient transformation)
}

pub enum MessageType
{
    /// A 480 * 270 buffer of rgb565 pixels
    DrawScreen(Vec<u8>),
    // other messsaage types, not relevant here
}

/// Instantiated in main and used to send events to the device
pub struct ControlInterface {
    /// Used to send messages to the device
    pub tx_pending_send: tokio::mpsc::Sender<Message>,
    // other fields left out, not relevant here
}
```

<details>
<summary>Response</summary>

To achieve capturing a 480x270 area centered around the mouse pointer at 60fps and sending it to your device, you can use the Windows GDI API to capture the screen area efficiently. Below is the code that accomplishes this task.

First, add the necessary dependencies to your Cargo.toml:

```
[dependencies]
image = "0.24"
tokio = { version = "1", features = ["full"] }
windows = { version = "0.48", features = ["Win32_Foundation", "Win32_Graphics_Gdi", "Win32_UI_WindowsAndMessaging"] }
```

Now, implement the required functions and the main streaming function:

```rust
use std::time::{Duration, Instant};
use tokio::task;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject,
    GetDIBits, SelectObject, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CAPTUREBLT, DIB_RGB_COLORS,
    HBITMAP, HDC, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, ReleaseDC};
use image::{DynamicImage, ImageBuffer, Rgba};

// Function to get the cursor position
fn get_cursor_pos() -> Result<(i32, i32), Box<dyn std::error::Error>> {
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point).as_bool() {
            Ok((point.x, point.y))
        } else {
            Err(std::io::Error::last_os_error().into())
        }
    }
}

// Function to capture a screen area and return it as a DynamicImage
fn capture_screen_area(x: i32, y: i32, width: i32, height: i32) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    unsafe {
        // Get the device context of the screen
        let hdc_screen: HDC = windows::Win32::Graphics::Gdi::GetDC(HWND(0));
        if hdc_screen.0 == 0 {
            return Err("Failed to get screen DC".into());
        }

        // Create a compatible DC which is used in a BitBlt from the window DC
        let hdc_mem_dc: HDC = CreateCompatibleDC(hdc_screen);
        if hdc_mem_dc.0 == 0 {
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible DC".into());
        }

        // Create a compatible bitmap from the Window DC
        let hbm_screen: HBITMAP = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbm_screen.0 == 0 {
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible bitmap".into());
        }

        // Select the compatible bitmap into the compatible memory DC
        let old_bmp = SelectObject(hdc_mem_dc, hbm_screen);
        if old_bmp.0 == 0 {
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to select object into DC".into());
        }

        // Bit block transfer into our compatible memory DC
        if !BitBlt(hdc_mem_dc, 0, 0, width, height, hdc_screen, x, y, SRCCOPY | CAPTUREBLT).as_bool() {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("BitBlt failed".into());
        }

        // Prepare bitmap info header
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative height to indicate a top-down DIB
                biPlanes: 1,
                biBitCount: 32, // 32 bits per pixel
                biCompression: BI_RGB as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        // Calculate the image size and create a buffer
        let image_size = (width * height * 4) as usize; // 4 bytes per pixel (RGBA)
        let mut buffer = vec![0u8; image_size];

        // Use GetDIBits to copy the image data into the buffer
        if GetDIBits(
            hdc_mem_dc,
            hbm_screen,
            0,
            height as u32,
            buffer.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        ) == 0 {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("GetDIBits failed".into());
        }

        // Convert the buffer into an ImageBuffer
        // The image data is in BGRA format, so we need to convert it to RGBA
        let mut image_buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
            .ok_or("Failed to create ImageBuffer")?;
        for pixel in image_buffer.pixels_mut() {
            let Rgba([b, g, r, a]) = *pixel;
            *pixel = Rgba([r, g, b, a]);
        }

        // Convert ImageBuffer to DynamicImage
        let image = DynamicImage::ImageRgba8(image_buffer);

        // Clean up
        SelectObject(hdc_mem_dc, old_bmp);
        DeleteObject(hbm_screen);
        DeleteDC(hdc_mem_dc);
        ReleaseDC(HWND(0), hdc_screen);

        Ok(image)
    }
}

// The main streaming function
async fn stream_screenshots(control_interface: ControlInterface) -> Result<(), Box<dyn std::error::Error>> {
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    loop {
        let start_time = Instant::now();

        // Spawn blocking task to capture and process the image
        let result = task::spawn_blocking(|| {
            // Get cursor position
            let (mouse_x, mouse_y) = get_cursor_pos()?;

            // Compute the rectangle to capture
            let x = mouse_x - (480 / 2);
            let y = mouse_y - (270 / 2);

            // Adjust x and y to ensure they are within screen bounds
            let x = x.max(0);
            let y = y.max(0);

            // Capture screen area
            let image = capture_screen_area(x, y, 480, 270)?;

            // Optionally resize or process the image here if needed

            // Convert image to rgb565
            let rgb565_data = image_to_rgb565(&image);

            Ok::<_, Box<dyn std::error::Error>>(rgb565_data)
        })
        .await?;

        match result {
            Ok(rgb565_data) => {
                // After getting the data, send it over the channel
                let message = MessageType::DrawScreen(rgb565_data);
                control_interface.tx_pending_send.send(message).await?;
            }
            Err(e) => {
                eprintln!("Error capturing screen: {}", e);
            }
        }

        // Sleep until the next frame
        let elapsed = start_time.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        } else {
            // We're behind schedule, no delay
        }
    }
}
```

## Explanation:

    - get_cursor_pos: Uses the Windows API to get the current mouse cursor position.
    - capture_screen_area: Captures the specified screen area using Windows GDI functions and converts the image data into an image::DynamicImage.
        It handles the image format conversion from BGRA (Windows default) to RGBA.
    - stream_screenshots: An asynchronous function that runs in a loop to capture the screen at 60fps.
        It uses tokio::task::spawn_blocking to offload the blocking screen capture and processing to a separate thread.
        Captured image data is sent to the device via the tokio::mpsc::Sender.
        Frame timing is managed to maintain approximately 60fps.

## Note:

    Error Handling: Errors are propagated using the Result type, and any errors during capture or processing are printed to stderr.
    Performance Considerations: Capturing the screen and processing image data can be intensive. Using spawn_blocking ensures that the asynchronous runtime is not blocked.
    Image Conversion: The captured screen data is converted from BGRA to RGBA format to be compatible with the image crate and your image_to_rgb565 function.

## Usage:

To use the stream_screenshots function, call it from an asynchronous context, such as within `#[tokio::main]`:

```
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup your ControlInterface and other initialization here

    // Start streaming screenshots
    stream_screenshots(control_interface).await?;

    Ok(())
}
```

This implementation should efficiently capture the screen area around the mouse pointer at the desired frame rate and send it to your device using the provided channel.
</details>

---
### Prompt 2

This works, however I would like it to show the mouse cursor as well. Right now it's hidden. Please show me the minimum number of updates to make to make this change.

<details>
<summary>Response</summary>

To include the mouse cursor in your captured screenshots, you'll need to update the capture_screen_area function to draw the cursor onto the captured image. The Windows GDI BitBlt function doesn't capture the cursor by default. To achieve this, you can use the GetCursorInfo function to retrieve the cursor's information and then use the DrawIconEx function to draw the cursor onto your in-memory device context (hdc_mem_dc).

Below are the minimal changes required to your existing code to include the mouse cursor in your screenshots.

1. Add Necessary Imports:

At the top of your file, include the additional Windows API functions and constants:

```rust
use windows::Win32::UI::WindowsAndMessaging::{
    CURSORINFO, GetCursorInfo, CopyIcon, DestroyIcon, DrawIconEx, CURSOR_SHOWING, DI_NORMAL,
};
```

2. Update the capture_screen_area Function:

Within your capture_screen_area function, after the BitBlt call and before GetDIBits, add the following code to capture and draw the cursor onto your bitmap.

```rust
// After BitBlt, add the following code:

// Get cursor info
let mut cursor_info = CURSORINFO {
    cbSize: std::mem::size_of::<CURSORINFO>() as u32,
    ..Default::default()
};

unsafe {
    if GetCursorInfo(&mut cursor_info).as_bool() && (cursor_info.flags & CURSOR_SHOWING.0) != 0 {
        // The cursor position in screen coordinates
        let cursor_x = cursor_info.ptScreenPos.x;
        let cursor_y = cursor_info.ptScreenPos.y;

        // Calculate cursor position relative to captured bitmap
        let cursor_x_in_bitmap = cursor_x - x;
        let cursor_y_in_bitmap = cursor_y - y;

        // Check if the cursor is within the captured area with a margin for cursor size
        if cursor_x_in_bitmap >= -32
            && cursor_x_in_bitmap <= width
            && cursor_y_in_bitmap >= -32
            && cursor_y_in_bitmap <= height
        {
            // Copy the cursor icon
            let hicon = CopyIcon(cursor_info.hCursor);
            if hicon.0 != 0 {
                // Draw the cursor onto the memory DC
                DrawIconEx(
                    hdc_mem_dc,
                    cursor_x_in_bitmap,
                    cursor_y_in_bitmap,
                    hicon,
                    0,
                    0,
                    0,
                    None,
                    DI_NORMAL,
                );

                // Destroy the icon after use
                DestroyIcon(hicon);
            }
        }
    }
}
```

## Explanation:

    - GetCursorInfo: Retrieves the cursor's position, visibility, and handle.
    - CURSORINFO: A structure that holds the cursor information.
    - CURSOR_SHOWING: A flag indicating whether the cursor is visible.
    - CopyIcon: Creates a copy of the cursor icon handle to ensure thread safety.
    - DrawIconEx: Draws the cursor icon onto the device context (hdc_mem_dc).
    - DestroyIcon: Releases the icon handle created by CopyIcon.

3. Handle Safety and Resource Management:

Ensure that all unsafe operations are within an unsafe block, and resources like icons are properly released to prevent leaks.

Updated capture_screen_area Function:

Here's how your capture_screen_area function should look with the minimal changes:

```rust
fn capture_screen_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    unsafe {
        // Get the device context of the screen
        let hdc_screen: HDC = windows::Win32::Graphics::Gdi::GetDC(HWND(0));
        if hdc_screen.0 == 0 {
            return Err("Failed to get screen DC".into());
        }

        // Create a compatible DC which is used in a BitBlt from the window DC
        let hdc_mem_dc: HDC = CreateCompatibleDC(hdc_screen);
        if hdc_mem_dc.0 == 0 {
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible DC".into());
        }

        // Create a compatible bitmap from the Window DC
        let hbm_screen: HBITMAP = CreateCompatibleBitmap(hdc_screen, width, height);
        if hbm_screen.0 == 0 {
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible bitmap".into());
        }

        // Select the compatible bitmap into the compatible memory DC
        let old_bmp = SelectObject(hdc_mem_dc, hbm_screen);
        if old_bmp.0 == 0 {
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to select object into DC".into());
        }

        // Bit block transfer into our compatible memory DC
        if !BitBlt(
            hdc_mem_dc,
            0,
            0,
            width,
            height,
            hdc_screen,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        )
        .as_bool()
        {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("BitBlt failed".into());
        }

        // Add this block to capture and draw the cursor
        // Get cursor info
        let mut cursor_info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };

        if GetCursorInfo(&mut cursor_info).as_bool() && (cursor_info.flags & CURSOR_SHOWING.0) != 0
        {
            // The cursor position in screen coordinates
            let cursor_x = cursor_info.ptScreenPos.x;
            let cursor_y = cursor_info.ptScreenPos.y;

            // Calculate cursor position relative to captured bitmap
            let cursor_x_in_bitmap = cursor_x - x;
            let cursor_y_in_bitmap = cursor_y - y;

            // Check if the cursor is within the captured area with a margin for cursor size
            if cursor_x_in_bitmap >= -32
                && cursor_x_in_bitmap <= width
                && cursor_y_in_bitmap >= -32
                && cursor_y_in_bitmap <= height
            {
                // Copy the cursor icon
                let hicon = CopyIcon(cursor_info.hCursor);
                if hicon.0 != 0 {
                    // Draw the cursor onto the memory DC
                    DrawIconEx(
                        hdc_mem_dc,
                        cursor_x_in_bitmap,
                        cursor_y_in_bitmap,
                        hicon,
                        0,
                        0,
                        0,
                        None,
                        DI_NORMAL,
                    );

                    // Destroy the icon after use
                    DestroyIcon(hicon);
                }
            }
        }

        // Proceed with existing code to convert the bitmap to an image

        // Prepare bitmap info header
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative height to indicate a top-down DIB
                biPlanes: 1,
                biBitCount: 32, // 32 bits per pixel
                biCompression: BI_RGB as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        // Calculate the image size and create a buffer
        let image_size = (width * height * 4) as usize; // 4 bytes per pixel (RGBA)
        let mut buffer = vec![0u8; image_size];

        // Use GetDIBits to copy the image data into the buffer
        if GetDIBits(
            hdc_mem_dc,
            hbm_screen,
            0,
            height as u32,
            buffer.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        ) == 0
        {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("GetDIBits failed".into());
        }

        // Convert the buffer into an ImageBuffer
        // The image data is in BGRA format, so we need to convert it to RGBA
        let mut image_buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, buffer)
                .ok_or("Failed to create ImageBuffer")?;
        for pixel in image_buffer.pixels_mut() {
            let Rgba([b, g, r, a]) = *pixel;
            *pixel = Rgba([r, g, b, a]);
        }

        // Convert ImageBuffer to DynamicImage
        let image = DynamicImage::ImageRgba8(image_buffer);

        // Clean up
        SelectObject(hdc_mem_dc, old_bmp);
        DeleteObject(hbm_screen);
        DeleteDC(hdc_mem_dc);
        ReleaseDC(HWND(0), hdc_screen);

        Ok(image)
    }
}
```

With these minimal changes, your screenshot captures will now include the mouse cursor.

## Note:

    - The cursor_x_in_bitmap and cursor_y_in_bitmap variables calculate the cursor's position relative to your captured area.
    - The additional checks ensure that the cursor is within the bounds of the captured image before attempting to draw it.
    - Using CopyIcon and DestroyIcon ensures that you handle the cursor icon correctly and prevent resource leaks.
    - The size of the cursor is typically around 32x32 pixels, so the margin in the checks accounts for this.

## Important Considerations:

    Ensure that all unsafe operations are properly contained within unsafe blocks.
    Be cautious with the types and handles when working with Windows API functions.
    The rest of your code, including the image processing and sending via the channel, remains unchanged.

By adding these minimal updates, your streaming screenshots will now display the mouse cursor, providing a complete view of the screen area as desired.

</details>

---
### Prompt 3
So I've noticed a problem with the earlier code: This doesn't actually center on the mouse cursor. When the mouse cursor is in the top-left of the screen, it's accurate. But as the mouse cursor moves down or right, the screenshot doesn't keep yp with it. It's impossible to view the far right or bottom of the screen due to this. Please diagnose and provide a solution to this issue.

<details>
<summary>Response</summary>

## Diagnosis:

The issue you're experiencing arises because the code doesn't adjust the capture area's position when the mouse cursor is near the right or bottom edges of the screen. Specifically, the code currently only ensures that x and y are not negative (not less than zero). It doesn't adjust for the scenario where x + width or y + height exceeds the screen's dimensions. As a result, when the mouse cursor moves towards the right or bottom edges, the capture area extends beyond the screen boundaries, and the screenshot doesn't remain centered on the cursor.

## Solution:

To fix this, we need to obtain the screen's width and height and adjust x and y accordingly to ensure that the capture area remains within the screen boundaries. We'll modify x and y to make sure:

    - x is between 0 and screen_width - width
    - y is between 0 and screen_height - height

This adjustment ensures that the capture area doesn't extend beyond the screen's edges and that the screenshot stays centered around the mouse cursor as much as possible, even when the cursor is near the edges.

### Minimal Code Updates:

    - Import the Necessary Function and Constants:

    - Add the following imports to access the screen dimensions using the GetSystemMetrics function:

```rust
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
```

Update the stream_screenshots Function:

Modify the computation of x and y to adjust for the screen dimensions. Here's the updated stream_screenshots function with the minimal changes:

```rust
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

async fn stream_screenshots(control_interface: ControlInterface) -> Result<(), Box<dyn std::error::Error>> {
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    loop {
        let start_time = Instant::now();

        // Spawn blocking task to capture and process the image
        let result = task::spawn_blocking(|| {
            // Get cursor position
            let (mouse_x, mouse_y) = get_cursor_pos()?;

            // Get screen dimensions
            let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

            // Compute the rectangle to capture
            let mut x = mouse_x - (480 / 2);
            let mut y = mouse_y - (270 / 2);

            // Adjust x and y to ensure they are within screen bounds
            x = x.max(0).min(screen_width - 480);
            y = y.max(0).min(screen_height - 270);

            // Capture screen area
            let image = capture_screen_area(x, y, 480, 270)?;

            // Convert image to rgb565
            let rgb565_data = image_to_rgb565(&image);

            Ok::<_, Box<dyn std::error::Error>>(rgb565_data)
        })
        .await?;

        match result {
            Ok(rgb565_data) => {
                // Send the data over the channel
                let message = MessageType::DrawScreen(rgb565_data);
                control_interface.tx_pending_send.send(message).await?;
            }
            Err(e) => {
                eprintln!("Error capturing screen: {}", e);
            }
        }

        // Sleep until the next frame
        let elapsed = start_time.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        } else {
            // We're behind schedule, no delay
        }
    }
}
```

## Explanation of Changes:

### Getting Screen Dimensions:

    We use GetSystemMetrics(SM_CXSCREEN) and GetSystemMetrics(SM_CYSCREEN) to obtain the screen's width and height.

```rust
let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
```

Note: Since GetSystemMetrics is an unsafe function, we wrap the calls in an unsafe block.

### Adjusting x and y:

We adjust x and y to ensure that the capture rectangle stays within the screen's boundaries.

        - x = x.max(0).min(screen_width - 480);
        - y = y.max(0).min(screen_height - 270);

        This clamps x and y between 0 and the maximum permissible values (screen_width - 480 for x and screen_height - 270 for y) so that the rectangle does not exceed the screen dimensions.

#### Why This Works:

    By adjusting x and y based on the screen dimensions, we ensure that:
        When the mouse cursor moves towards the right edge, and x + 480 would exceed screen_width, we reduce x to screen_width - 480. This keeps the right edge of the capture area within the screen.
        Similarly, when moving towards the bottom edge, we adjust y to keep the bottom edge of the capture area within the screen.

    This adjustment allows the captured area to remain centered around the mouse cursor as much as possible without going beyond the screen limits.

## Example Scenario:

- Screen Resolution: 1920x1080

- Capture Area Size: 480x270

Mouse Positions and Adjustments:
|Mouse Position (x, y)|Initial x (mouse_x - 240)|Adjusted x (x.max(0).min(1440))|Initial y (mouse_y - 135)|Adjusted y (y.max(0).min(810))|
|-|-|-|-|-|
|(100, 100) |-140 |0 |-35 |0|
|(1900, 100) |1660 |1440 |-35 |0|
|(100, 1000) |-140 |0 |865 |810|
|(1900, 1000) |1660|1440 |865|810|

In all cases, the adjusted x and y ensure that the capture area stays within the screen boundaries.

## Additional Notes:

    Multi-Monitor Setups:

    If you're working with multiple monitors, you may need to handle virtual screen coordinates or get the monitor dimensions where the cursor is currently located. The GetSystemMetrics function with SM_CXSCREEN and SM_CYSCREEN retrieves the primary monitor's dimensions. For multi-monitor support, consider using EnumDisplayMonitors or related functions to get accurate dimensions.

    Error Handling:

    Ensure you handle cases where GetSystemMetrics might fail, although it's unlikely. Always consider adding checks or handling edge cases for robustness.

## Conclusion:

By making these minimal changes, your screenshot captures will now remain centered on the mouse cursor, even when it moves towards the right or bottom edges of the screen. This adjustment ensures that the entire capture area stays within the screen boundaries, providing the expected behavior.

</details>

---
### Prompt 4
This doesn't solve the original issue: x and y still appear to pick incorrect areas of the screen in most situations. Why is this, and what is the solution?

<details>
<summary>Response</summary>

## Diagnosis:

The issue persists because the original code doesn't correctly handle cases where the mouse cursor moves near or beyond the edges of the primary monitor, especially in multi-monitor setups. Specifically:

    Virtual Screen Coordinates: The cursor position obtained from GetCursorPos is in virtual screen coordinates that can be negative or extend beyond the dimensions of the primary monitor when multiple monitors are involved.

    Screen Dimensions: Using GetSystemMetrics(SM_CXSCREEN) and SM_CYSCREEN only retrieves the dimensions of the primary monitor, not the entire virtual screen that spans all monitors.

    Capture Area Calculations: The code doesn't correctly adjust the capture area (x, y, width, height) when the desired capture area extends beyond the edges of the virtual screen. This leads to incorrect capturing when the cursor is near the edges.

## Solution:

To fix this issue, we need to:

    Use Virtual Screen Metrics: Replace SM_CXSCREEN and SM_CYSCREEN with SM_CXVIRTUALSCREEN and SM_CYVIRTUALSCREEN to get the dimensions of the virtual screen that encompasses all monitors.

    Adjust for Virtual Screen Origin: Incorporate SM_XVIRTUALSCREEN and SM_YVIRTUALSCREEN to get the top-left coordinates of the virtual screen, which may not be (0, 0) in multi-monitor setups.

    Correctly Calculate the Capture Area: When the desired capture area extends beyond the virtual screen boundaries, adjust the capture parameters to ensure we only capture the valid portions within the virtual screen.

    Update the capture_screen_area Function: Modify this function to accept destination offsets and adjust the capture accordingly.

## Minimal Code Updates:

Below are the minimal changes required to fix the issue.

1. Update Imports:

Add the virtual screen constants to your imports:

```rust
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
```

2. Update the stream_screenshots Function:

Modify the code in your stream_screenshots function to use virtual screen metrics and adjust the capture area calculations:

```rust
async fn stream_screenshots(control_interface: ControlInterface) -> Result<(), Box<dyn std::error::Error>> {
    let frame_duration = Duration::from_secs_f64(1.0 / 60.0);

    loop {
        let start_time = Instant::now();

        // Spawn blocking task to capture and process the image
        let result = task::spawn_blocking(|| {
            // Get cursor position
            let (mouse_x, mouse_y) = get_cursor_pos()?;

            // Get virtual screen dimensions
            let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
            let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
            let virtual_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
            let virtual_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

            // Desired capture dimensions
            let capture_width = 480;
            let capture_height = 270;

            // Compute the desired capture rectangle centered around the mouse
            let desired_x = mouse_x - (capture_width / 2);
            let desired_y = mouse_y - (capture_height / 2);

            // Compute the intersection of the desired capture rectangle with the virtual screen
            let capture_left = desired_x.max(virtual_left);
            let capture_top = desired_y.max(virtual_top);
            let capture_right = (desired_x + capture_width).min(virtual_left + virtual_width);
            let capture_bottom = (desired_y + capture_height).min(virtual_top + virtual_height);

            // Adjust width and height based on the intersection
            let adjusted_width = (capture_right - capture_left).max(0);
            let adjusted_height = (capture_bottom - capture_top).max(0);

            // If adjusted dimensions are zero or negative, there's nothing to capture
            if adjusted_width <= 0 || adjusted_height <= 0 {
                return Err("No valid screen area to capture".into());
            }

            // Calculate the destination offsets if the capture area is smaller than desired dimensions
            let dest_x = (capture_left - desired_x) as i32;
            let dest_y = (capture_top - desired_y) as i32;

            // Capture screen area
            let image = capture_screen_area(
                capture_left,
                capture_top,
                adjusted_width as i32,
                adjusted_height as i32,
                dest_x,
                dest_y,
                capture_width as i32,
                capture_height as i32,
            )?;

            // Convert image to rgb565
            let rgb565_data = image_to_rgb565(&image);

            Ok::<_, Box<dyn std::error::Error>>(rgb565_data)
        })
        .await?;

        match result {
            Ok(rgb565_data) => {
                // Send the data over the channel
                let message = MessageType::DrawScreen(rgb565_data);
                control_interface.tx_pending_send.send(message).await?;
            }
            Err(e) => {
                eprintln!("Error capturing screen: {}", e);
            }
        }

        // Sleep until the next frame
        let elapsed = start_time.elapsed();
        if elapsed < frame_duration {
            tokio::time::sleep(frame_duration - elapsed).await;
        } else {
            // We're behind schedule, no delay
        }
    }
}
```

## Explanation:

    Virtual Screen Coordinates: We retrieve the virtual screen's left and top coordinates (virtual_left, virtual_top), which may be negative.

    Desired Capture Rectangle: We calculate desired_x and desired_y to center the capture area around the mouse cursor.

    Intersection with Virtual Screen: We find the intersection of the desired capture rectangle with the virtual screen to ensure we only capture valid screen areas.

    Adjusted Width and Height: By calculating adjusted_width and adjusted_height, we handle cases where part of the desired capture area is off-screen.

    Destination Offsets: dest_x and dest_y determine where the captured image should be placed within the final image buffer. This ensures the captured content is correctly aligned even when it's smaller than the desired dimensions.

3. Update the capture_screen_area Function:

Modify the capture_screen_area function to accept dest_x, dest_y, dest_width, and dest_height parameters. Adjust the function to handle these parameters:

```rust
fn capture_screen_area(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    dest_x: i32,
    dest_y: i32,
    dest_width: i32,
    dest_height: i32,
) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    unsafe {
        // Get the device context of the screen
        let hdc_screen: HDC = windows::Win32::Graphics::Gdi::GetDC(HWND(0));
        if hdc_screen.0 == 0 {
            return Err("Failed to get screen DC".into());
        }

        // Create a compatible DC for screen
        let hdc_mem_dc: HDC = CreateCompatibleDC(hdc_screen);
        if hdc_mem_dc.0 == 0 {
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible DC".into());
        }

        // Create a bitmap with desired dimensions
        let hbm_screen: HBITMAP = CreateCompatibleBitmap(hdc_screen, dest_width, dest_height);
        if hbm_screen.0 == 0 {
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to create compatible bitmap".into());
        }

        // Select the bitmap into the DC
        let old_bmp = SelectObject(hdc_mem_dc, hbm_screen);
        if old_bmp.0 == 0 {
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("Failed to select object into DC".into());
        }

        // Optional: Fill the bitmap with black color
        windows::Win32::Graphics::Gdi::PatBlt(
            hdc_mem_dc,
            0,
            0,
            dest_width,
            dest_height,
            windows::Win32::Graphics::Gdi::BLACKNESS,
        );

        // Bit block transfer the adjusted capture area into the memory DC at the correct offset
        if !BitBlt(
            hdc_mem_dc,
            dest_x,
            dest_y,
            width,
            height,
            hdc_screen,
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        )
        .as_bool()
        {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("BitBlt failed".into());
        }

        // Get cursor info
        let mut cursor_info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };

        if GetCursorInfo(&mut cursor_info).as_bool() && (cursor_info.flags & CURSOR_SHOWING.0) != 0
        {
            // The cursor position in screen coordinates
            let cursor_x = cursor_info.ptScreenPos.x;
            let cursor_y = cursor_info.ptScreenPos.y;

            // Calculate cursor position relative to the bitmap
            let cursor_x_in_bitmap = cursor_x - x + dest_x;
            let cursor_y_in_bitmap = cursor_y - y + dest_y;

            // Check if the cursor is within the bitmap
            if cursor_x_in_bitmap >= 0
                && cursor_x_in_bitmap <= dest_width
                && cursor_y_in_bitmap >= 0
                && cursor_y_in_bitmap <= dest_height
            {
                // Copy the cursor icon
                let hicon = CopyIcon(cursor_info.hCursor);
                if hicon.0 != 0 {
                    // Draw the cursor onto the memory DC
                    DrawIconEx(
                        hdc_mem_dc,
                        cursor_x_in_bitmap,
                        cursor_y_in_bitmap,
                        hicon,
                        0,
                        0,
                        0,
                        None,
                        DI_NORMAL,
                    );

                    // Destroy the icon after use
                    DestroyIcon(hicon);
                }
            }
        }

        // Prepare bitmap info header
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: dest_width,
                biHeight: -dest_height, // Negative height for top-down DIB
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB as u32,
                ..Default::default()
            },
            ..Default::default()
        };

        // Calculate the image size and create a buffer
        let image_size = (dest_width * dest_height * 4) as usize; // 4 bytes per pixel (RGBA)
        let mut buffer = vec![0u8; image_size];

        // Use GetDIBits to copy the image data into the buffer
        if GetDIBits(
            hdc_mem_dc,
            hbm_screen,
            0,
            dest_height as u32,
            buffer.as_mut_ptr() as *mut _,
            &mut bmi,
            DIB_RGB_COLORS,
        ) == 0
        {
            SelectObject(hdc_mem_dc, old_bmp);
            DeleteObject(hbm_screen);
            DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(0), hdc_screen);
            return Err("GetDIBits failed".into());
        }

        // Convert the buffer into an ImageBuffer
        // The image data is in BGRA format, so we need to convert it to RGBA
        let mut image_buffer =
            ImageBuffer::<Rgba<u8>, _>::from_raw(dest_width as u32, dest_height as u32, buffer)
                .ok_or("Failed to create ImageBuffer")?;
        for pixel in image_buffer.pixels_mut() {
            let Rgba([b, g, r, a]) = *pixel;
            *pixel = Rgba([r, g, b, a]);
        }

        // Convert ImageBuffer to DynamicImage
        let image = DynamicImage::ImageRgba8(image_buffer);

        // Clean up
        SelectObject(hdc_mem_dc, old_bmp);
        DeleteObject(hbm_screen);
        DeleteDC(hdc_mem_dc);
        ReleaseDC(HWND(0), hdc_screen);

        Ok(image)
    }
}
```

## Explanation:

    Destination Dimensions: The bitmap is created using dest_width and dest_height to ensure it always matches the expected output size (480x270).

    Destination Offsets in BitBlt: The dest_x and dest_y parameters in BitBlt place the captured image at the correct position within the destination bitmap.

    Cursor Position Adjustment: The cursor's position within the bitmap is adjusted using dest_x and dest_y to account for any offsets.

    Handling Negative Coordinates: By adjusting x, y, width, and height based on the intersection with the virtual screen, we ensure BitBlt doesn't attempt to read pixels outside the screen boundaries.

4. Update the Cursor Drawing Logic:

Ensure the cursor drawing code accounts for the destination offsets:

```rust
// Adjusted cursor position relative to the bitmap
let cursor_x_in_bitmap = cursor_x - x + dest_x;
let cursor_y_in_bitmap = cursor_y - y + dest_y;
```

## Notes:

    Multi-Monitor Support: By using virtual screen metrics and handling negative coordinates, the code now correctly supports multiple monitors and ensures the capture area follows the cursor across all monitors.

    Edge Cases: The adjusted capture area calculations handle situations where the cursor is near the edges of the virtual screen, ensuring the capture area doesn't extend beyond available screen space.

    Performance Considerations: The minimal changes focus on correcting the capture logic without significantly altering the structure or performance characteristics of the code.

## Summary:

    Issue: The original code didn't handle virtual screen coordinates and multiple monitors, causing incorrect capture areas when the cursor moved towards the edges of the screen.

    Solution: Adjust the capture calculations to use virtual screen metrics and handle negative coordinates, ensuring the capture area always aligns correctly with the mouse cursor.

By making these minimal updates, your program should now correctly capture a 480x270 area centered around the mouse cursor, including when the cursor is near the edges of the screen or across multiple monitors.

</details>

---
### Prompt 5
That's an improvement, but there is still a problem: the rendered screenshot seems to cut off at about 1.5x the actual screen height and width. When the cursor is 100% to the right, the rendered image is only roughly 75% to the right. When the cursor is 50% to the right, the rendered image is only roughly 33% to the right. Same issue for height. (The percentages here are rough guesses, but hopefully can help explain the issue). Can you diagnose and provide a solution for this issue?

<details>
<summary>Response</summary>

## Diagnosis:

The issue you're encountering is likely due to DPI scaling on your Windows system. When an application is not DPI-aware, Windows provides scaled (logical) coordinates to the application, which can cause discrepancies in coordinate calculations. This scaling affects cursor positions and screen dimensions retrieved via Windows API functions, leading to the proportional misalignment you're observing.

Specifically:

    High DPI Displays and Scaling Factors: If your display scaling is set to something other than 100% (e.g., 125%, 150%), Windows scales the coordinates accordingly for applications that are not DPI-aware.

    Coordinate Mismatch: The GetCursorPos and GetSystemMetrics functions may return coordinates that are scaled differently, causing the captured area to not align correctly with the actual cursor position.

    Proportional Misalignment: The proportional nature of the misalignment (e.g., when the cursor is at 50%, the image is at 33%) suggests that the scaling factor is affecting the coordinates.

## Solution:

To resolve this issue, you need to make your application DPI-aware. This ensures that you receive true physical pixel coordinates rather than scaled logical coordinates, and that all coordinate calculations remain consistent.

Minimal Code Updates:

    Import the SetProcessDPIAware Function:

    At the top of your file, import the SetProcessDPIAware function from the Windows API:

```rust
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;
```

Call SetProcessDPIAware at the Start of Your Application:

In your main function or before any other code runs, call SetProcessDPIAware to declare your process as DPI-aware:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Call SetProcessDPIAware to ensure correct DPI handling
    unsafe {
        SetProcessDPIAware();
    }

    // Setup your ControlInterface and other initialization here

    // Start streaming screenshots
    stream_screenshots(control_interface).await?;

    Ok(())
}
```

    Note: The SetProcessDPIAware function must be called before any UI-related code runs. In this case, calling it at the very beginning of main ensures it takes effect.

## Explanation:

    DPI Awareness: Declaring your application as DPI-aware tells Windows that your application handles DPI scaling appropriately and does not need coordinate virtualization or automatic scaling.

    Consistent Coordinates: After making your process DPI-aware, functions like GetCursorPos and GetSystemMetrics will return coordinates in physical pixels, ensuring consistency across all calculations.

    No Additional Changes Required: This minimal change should resolve the issue without needing to adjust your existing coordinate calculations or modify your image capturing logic.

## Additional Considerations:

    High DPI Displays: With the increasing prevalence of high DPI displays, it's essential for applications that handle pixel-level operations (like screen capturing) to be DPI-aware.

    Testing: After making this change, test your application on systems with different scaling settings (e.g., 100%, 125%, 150%) to ensure the issue is resolved across various configurations.

    Manifest-Based DPI Awareness (Optional):

    Alternatively, you can declare DPI awareness in your application's manifest file by adding the following XML:

    <application xmlns="urn:schemas-microsoft-com:asm.v3">
      <windowsSettings>
        <dpiAwareness>PerMonitorV2</dpiAwareness>
      </windowsSettings>
    </application>

    However, managing application manifests can be more complex, especially in Rust applications. Using SetProcessDPIAware is a straightforward solution.

Summary of Steps:

    Import the Function:

```rust
use windows::Win32::UI::WindowsAndMessaging::SetProcessDPIAware;
```

Call the Function in main:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        SetProcessDPIAware();
    }

    // ... rest of your code ...
}
```

By making your application DPI-aware, you ensure that all coordinate calculations are accurate and consistent, resolving the misalignment issue you're experiencing. This minimal change should fix the problem without requiring significant modifications to your existing code.

Final Note:

If, after making this change, you still experience issues, consider adding logging or debug statements to output the values of mouse_x, mouse_y, and the capture coordinates. This can help verify that the coordinates match your expectations and further diagnose any remaining discrepancies.

</details>

---
### Prompt 6
Ok, that fixed it. Now for the next problem: The mouse cursor renders a few pixels off from where it actually is on the screen. What's the cause and how can I fix this?

<details>
<summary>Response</summary>

## Diagnosis:

The issue you're experiencing—where the mouse cursor renders a few pixels off from its actual position—is due to not accounting for the cursor's hotspot when drawing it onto the captured image. The hotspot is the specific point within the cursor image that represents the actual click point or selection point on the screen. For example, in the default arrow cursor, the hotspot is at the tip of the arrow, not the top-left corner of the cursor image.

In the current implementation, the cursor is drawn onto the capture without adjusting for the hotspot, so the cursor image appears offset from its true position.

## Solution:

To fix this issue, you need to retrieve the cursor's hotspot coordinates and adjust the position where you draw the cursor onto the captured image. You can achieve this by using the GetIconInfo function, which retrieves information about the icon or cursor, including the hotspot coordinates.

Minimal Code Updates:

Here are the minimal changes required to adjust the cursor drawing to account for the hotspot.

1. Import Necessary Functions and Structures:

At the top of your file, add the following imports:

```rust
use windows::Win32::Graphics::Gdi::{GetIconInfo, ICONINFO, DeleteObject};
```

2. Update the Cursor Drawing Logic in capture_screen_area:

In your capture_screen_area function, modify the cursor drawing code to retrieve the hotspot and adjust the cursor's position.

Before:

```rust
// Existing code to calculate cursor position within bitmap
let cursor_x_in_bitmap = cursor_x - x + dest_x;
let cursor_y_in_bitmap = cursor_y - y + dest_y;

// Draw the cursor onto the memory DC
DrawIconEx(
    hdc_mem_dc,
    cursor_x_in_bitmap,
    cursor_y_in_bitmap,
    hicon,
    0,
    0,
    0,
    None,
    DI_NORMAL,
);
```

After:

```rust
// Copy the cursor icon
let hicon = CopyIcon(cursor_info.hCursor);
if hicon.0 != 0 {
    // Get the cursor's hotspot
    let mut icon_info = ICONINFO::default();
    if GetIconInfo(hicon, &mut icon_info).as_bool() {
        let hotspot_x = icon_info.xHotspot as i32;
        let hotspot_y = icon_info.yHotspot as i32;

        // Correct the cursor position based on the hotspot
        let cursor_x_in_bitmap = cursor_x - x + dest_x - hotspot_x;
        let cursor_y_in_bitmap = cursor_y - y + dest_y - hotspot_y;

        // Draw the cursor onto the memory DC
        DrawIconEx(
            hdc_mem_dc,
            cursor_x_in_bitmap,
            cursor_y_in_bitmap,
            hicon,
            0,
            0,
            0,
            None,
            DI_NORMAL,
        );

        // Clean up icon info bitmaps
        if icon_info.hbmMask.0 != 0 {
            DeleteObject(icon_info.hbmMask);
        }
        if icon_info.hbmColor.0 != 0 {
            DeleteObject(icon_info.hbmColor);
        }
    } else {
        // If GetIconInfo fails, fallback to previous calculation
        let cursor_x_in_bitmap = cursor_x - x + dest_x;
        let cursor_y_in_bitmap = cursor_y - y + dest_y;

        // Draw the cursor onto the memory DC
        DrawIconEx(
            hdc_mem_dc,
            cursor_x_in_bitmap,
            cursor_y_in_bitmap,
            hicon,
            0,
            0,
            0,
            None,
            DI_NORMAL,
        );
    }

    // Destroy the icon after use
    DestroyIcon(hicon);
}
```

Explanation of Changes:

    Retrieve the Cursor's Hotspot:

```rust
let mut icon_info = ICONINFO::default();
if GetIconInfo(hicon, &mut icon_info).as_bool() {
    let hotspot_x = icon_info.xHotspot as i32;
    let hotspot_y = icon_info.yHotspot as i32;
    // ...
}
```

    GetIconInfo fills the ICONINFO structure with information about the cursor, including the hotspot coordinates.
    The xHotspot and yHotspot fields represent the offset from the top-left corner of the cursor image to the hotspot.

Adjust the Cursor Position:
```rust
let cursor_x_in_bitmap = cursor_x - x + dest_x - hotspot_x;
let cursor_y_in_bitmap = cursor_y - y + dest_y - hotspot_y;
```
    By subtracting the hotspot coordinates, you align the cursor image's hotspot with the cursor's actual screen position.
    This ensures the cursor appears at the correct position in the captured image.

Clean Up Resources:

```rust
if icon_info.hbmMask.0 != 0 {
    DeleteObject(icon_info.hbmMask);
}
if icon_info.hbmColor.0 != 0 {
    DeleteObject(icon_info.hbmColor);
}
```
        ICONINFO may allocate bitmaps for hbmMask and hbmColor, which need to be deleted using DeleteObject to prevent resource leaks.
        Always check if the handles are not null (0) before attempting to delete them.

    Fallback if GetIconInfo Fails:

    If GetIconInfo fails for some reason, the code falls back to the previous calculation without the hotspot adjustment. This ensures that the cursor is still drawn, albeit possibly slightly offset.

Complete Updated Cursor Drawing Code:

Here's how the cursor drawing portion of your capture_screen_area function should look:

```rust
// Get cursor info
let mut cursor_info = CURSORINFO {
    cbSize: std::mem::size_of::<CURSORINFO>() as u32,
    ..Default::default()
};

if GetCursorInfo(&mut cursor_info).as_bool() && (cursor_info.flags & CURSOR_SHOWING.0) != 0
{
    // The cursor position in screen coordinates
    let cursor_x = cursor_info.ptScreenPos.x;
    let cursor_y = cursor_info.ptScreenPos.y;

    // Copy the cursor icon
    let hicon = CopyIcon(cursor_info.hCursor);
    if hicon.0 != 0 {
        // Get the cursor's hotspot
        let mut icon_info = ICONINFO::default();
        if GetIconInfo(hicon, &mut icon_info).as_bool() {
            let hotspot_x = icon_info.xHotspot as i32;
            let hotspot_y = icon_info.yHotspot as i32;

            // Correct the cursor position based on the hotspot
            let cursor_x_in_bitmap = cursor_x - x + dest_x - hotspot_x;
            let cursor_y_in_bitmap = cursor_y - y + dest_y - hotspot_y;

            // Draw the cursor onto the memory DC
            DrawIconEx(
                hdc_mem_dc,
                cursor_x_in_bitmap,
                cursor_y_in_bitmap,
                hicon,
                0,
                0,
                0,
                None,
                DI_NORMAL,
            );

            // Clean up icon info bitmaps
            if icon_info.hbmMask.0 != 0 {
                DeleteObject(icon_info.hbmMask);
            }
            if icon_info.hbmColor.0 != 0 {
                DeleteObject(icon_info.hbmColor);
            }
        } else {
            // If GetIconInfo fails, fallback to previous calculation
            let cursor_x_in_bitmap = cursor_x - x + dest_x;
            let cursor_y_in_bitmap = cursor_y - y + dest_y;

            // Draw the cursor onto the memory DC
            DrawIconEx(
                hdc_mem_dc,
                cursor_x_in_bitmap,
                cursor_y_in_bitmap,
                hicon,
                0,
                0,
                0,
                None,
                DI_NORMAL,
            );
        }

        // Destroy the icon after use
        DestroyIcon(hicon);
    }
}
```

Notes:

    Resource Management:
        Always ensure that resources allocated by Windows API functions are properly released.
        Failing to delete the bitmap handles from ICONINFO can lead to resource leaks over time.

    Error Handling:
        The code checks whether GetIconInfo succeeds and provides a fallback in case it fails.
        This ensures robustness and that your application continues to function even in edge cases.

    Hotspot Coordinates:
        The hotspot coordinates (xHotspot, yHotspot) are typically within the cursor image's dimensions.
        By adjusting for the hotspot, you align the cursor image so that its hotspot aligns with the cursor's actual position.

Testing the Changes:

After implementing these changes, test your application by moving the cursor around the screen, including to the edges, and verify that the cursor appears at the correct position within the captured image.

Additional Considerations:

    Custom Cursors:
        If your application or system uses custom cursors with different hotspot positions, this adjustment becomes even more critical.
        The code accounts for any cursor, regardless of its hotspot position.

    High DPI Displays:
        Ensure that you've made your application DPI-aware (as discussed previously) before applying these changes.
        This ensures that cursor positions and dimensions are consistent across different DPI settings.

Summary:

    Issue: The cursor appears offset because the hotspot—the point within the cursor image that represents its actual position—is not accounted for in the drawing calculations.

    Solution: Retrieve the cursor's hotspot using GetIconInfo and adjust the drawing position by subtracting the hotspot coordinates.

    Minimal Code Changes: Import GetIconInfo, update the cursor drawing logic to adjust for the hotspot, and ensure proper resource cleanup.

By making these minimal updates, your application should now render the mouse cursor at the correct position within the captured images, providing an accurate representation of the screen area centered around the mouse pointer.

</details>