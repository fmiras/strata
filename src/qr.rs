use qrcode::QrCode;

/// Print a QR code to the console using Unicode block characters
pub fn print_qr_code(data: &str) {
    let code = match QrCode::new(data) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Warning: Could not generate QR code");
            return;
        }
    };

    let colors = code.to_colors();
    let width = code.width();

    // Use Unicode half-block characters for compact QR display
    // Upper half block: \u{2580}, Lower half block: \u{2584}, Full block: \u{2588}
    // We process two rows at a time using half blocks

    // Add quiet zone (white border)
    let quiet_zone = 1;
    let full_width = width + quiet_zone * 2;

    // Print top quiet zone
    for _ in 0..quiet_zone / 2 + 1 {
        println!("{}", " ".repeat(full_width));
    }

    for row in (0..width).step_by(2) {
        let mut line = " ".repeat(quiet_zone); // Left quiet zone

        for col in 0..width {
            let top = colors[row * width + col];
            let bottom = if row + 1 < width {
                colors[(row + 1) * width + col]
            } else {
                qrcode::Color::Light // Treat as white if odd height
            };

            // Dark = black (the QR modules), Light = white (background)
            let ch = match (top, bottom) {
                (qrcode::Color::Dark, qrcode::Color::Dark) => "\u{2588}",   // Full block (both dark)
                (qrcode::Color::Dark, qrcode::Color::Light) => "\u{2580}", // Upper half (top dark)
                (qrcode::Color::Light, qrcode::Color::Dark) => "\u{2584}", // Lower half (bottom dark)
                (qrcode::Color::Light, qrcode::Color::Light) => " ",       // Space (both light)
            };
            line.push_str(ch);
        }

        line.push_str(&" ".repeat(quiet_zone)); // Right quiet zone
        println!("{}", line);
    }

    // Print bottom quiet zone
    for _ in 0..quiet_zone / 2 + 1 {
        println!("{}", " ".repeat(full_width));
    }
}
