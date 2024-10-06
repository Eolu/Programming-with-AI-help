use image::{DynamicImage, ImageBuffer, Rgba};
use std::time::{Duration, Instant};
use stream_controller_rs::{image_to_rgb565, ControlInterface, Message, MessageType};
use tokio::task;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HBITMAP, HDC, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, DestroyIcon, DrawIconEx, GetCursorInfo, CURSORINFO, CURSOR_SHOWING, DI_NORMAL,
};
use windows::Win32::UI::WindowsAndMessaging::{GetIconInfo, ICONINFO};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

const FRAME_DURATION: Duration = Duration::from_millis(10);

// Function to get the cursor position
fn get_cursor_pos() -> Result<(i32, i32), Box<dyn std::error::Error>> {
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point).unwrap();
        Ok((point.x, point.y))
    }
}

// Function to capture a screen area and return it as a DynamicImage
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
        let hdc_screen: HDC = windows::Win32::Graphics::Gdi::GetDC(HWND(std::ptr::null_mut()));
        if hdc_screen.is_invalid() {
            return Err("Failed to get screen DC".into());
        }

        // Create a compatible DC for screen
        let hdc_mem_dc = CreateCompatibleDC(hdc_screen);
        if hdc_mem_dc.is_invalid() {
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err("Failed to create compatible DC".into());
        }

        // Create a bitmap with desired dimensions
        let hbm_screen: HBITMAP = CreateCompatibleBitmap(hdc_screen, dest_width, dest_height);
        if hbm_screen.is_invalid() {
            let _ = DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err("Failed to create compatible bitmap".into());
        }

        // Select the bitmap into the DC
        let old_bmp = SelectObject(hdc_mem_dc, hbm_screen);
        if old_bmp.is_invalid() {
            let _ = DeleteObject(hbm_screen);
            let _ = DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err("Failed to select object into DC".into());
        }

        // Optional: Fill the bitmap with black color
        let _ = windows::Win32::Graphics::Gdi::PatBlt(
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
        .is_ok()
        {
            SelectObject(hdc_mem_dc, old_bmp);
            let _ = DeleteObject(hbm_screen);
            let _ = DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
            return Err("BitBlt failed".into());
        }

        // Get cursor info
        let mut cursor_info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };

        if GetCursorInfo(&mut cursor_info).is_ok() && (cursor_info.flags.0 & CURSOR_SHOWING.0) != 0
        {
            // The cursor position in screen coordinates
            let cursor_x = cursor_info.ptScreenPos.x;
            let cursor_y = cursor_info.ptScreenPos.y;

            // Copy the cursor icon
            let hicon = CopyIcon(cursor_info.hCursor).unwrap();
            if !hicon.is_invalid() {
                // Get the cursor's hotspot
                let mut icon_info = ICONINFO::default();
                if GetIconInfo(hicon, &mut icon_info).is_ok() {
                    let hotspot_x = icon_info.xHotspot as i32;
                    let hotspot_y = icon_info.yHotspot as i32;

                    // Correct the cursor position based on the hotspot
                    let cursor_x_in_bitmap = cursor_x - x + dest_x - hotspot_x;
                    let cursor_y_in_bitmap = cursor_y - y + dest_y - hotspot_y;

                    // Draw the cursor onto the memory DC
                    let _ = DrawIconEx(
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
                    if !icon_info.hbmMask.is_invalid() {
                        let _ = DeleteObject(icon_info.hbmMask);
                    }
                    if !icon_info.hbmColor.is_invalid() {
                        let _ = DeleteObject(icon_info.hbmColor);
                    }
                } else {
                    // If GetIconInfo fails, fallback to previous calculation
                    let cursor_x_in_bitmap = cursor_x - x + dest_x;
                    let cursor_y_in_bitmap = cursor_y - y + dest_y;

                    // Draw the cursor onto the memory DC
                    let _ = DrawIconEx(
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
                let _ = DestroyIcon(hicon);
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
                biCompression: BI_RGB.0 as u32,
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
            Some(buffer.as_mut_ptr() as *mut _),
            &mut bmi,
            DIB_RGB_COLORS,
        ) == 0
        {
            SelectObject(hdc_mem_dc, old_bmp);
            let _ = DeleteObject(hbm_screen);
            let _ = DeleteDC(hdc_mem_dc);
            ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);
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
        let _ = DeleteObject(hbm_screen);
        let _ = DeleteDC(hdc_mem_dc);
        ReleaseDC(HWND(std::ptr::null_mut()), hdc_screen);

        Ok(image)
    }
}

// The main streaming function
pub(crate) async fn stream_screenshot(control_interface: &ControlInterface) -> std::io::Result<()> {
    let start_time = Instant::now();

    // Spawn blocking task to capture and process the image
    let result = task::spawn_blocking(|| {
        // Get cursor position
        let (mouse_x, mouse_y) = get_cursor_pos().unwrap();
        // println!("Cursor pos: ({mouse_x},{mouse_y})");

        // Get virtual screen dimensions
        let virtual_left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let virtual_top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let virtual_width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let virtual_height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        // Desired capture dimensions
        const CAPTURE_WIDTH: i32 = 480;
        const CAPTURE_HEIGHT: i32 = 270;

        // Compute the desired capture rectangle centered around the mouse
        let desired_x = mouse_x - (CAPTURE_WIDTH / 2);
        let desired_y = mouse_y - (CAPTURE_HEIGHT / 2);

        // Compute the intersection of the desired capture rectangle with the virtual screen
        let capture_left = desired_x.max(virtual_left);
        let capture_top = desired_y.max(virtual_top);
        let capture_right = (desired_x + CAPTURE_WIDTH).min(virtual_left + virtual_width);
        let capture_bottom = (desired_y + CAPTURE_HEIGHT).min(virtual_top + virtual_height);

        // Adjust width and height based on the intersection
        let adjusted_width = (capture_right - capture_left).max(0);
        let adjusted_height = (capture_bottom - capture_top).max(0);

        // If adjusted dimensions are zero or negative, there's nothing to capture
        if adjusted_width <= 0 || adjusted_height <= 0 {
            panic!("No valid screen area to capture");
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
            CAPTURE_WIDTH as i32,
            CAPTURE_HEIGHT as i32,
        )
        .unwrap();

        // Optionally resize or process the image here if needed

        // Convert image to rgb565
        let rgb565_data = image_to_rgb565(&image);

        rgb565_data
    })
    .await?;

    let msg = Message {
        mtype: MessageType::DrawScreen(result),
        tx_id: 1,
    };
    tokio::select! {
        _ = control_interface.tx_pending_send.send(msg) => (),
        _ = control_interface.shutdown_token.cancelled() => return Ok(()),
    }

    // Sleep until the next frame
    let elapsed = start_time.elapsed();
    if elapsed < FRAME_DURATION {
        tokio::time::sleep(FRAME_DURATION - elapsed).await;
    } else {
        // We're behind schedule, no delay
    }
    Ok(())
}
