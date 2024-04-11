#![no_std]
#![no_main]

use core::cell::RefCell;

use defmt::*;
// use display_interface_spi::{SPIInterface, SPIInterfaceNoCS};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::spi;
use embassy_rp::spi::{Blocking, Spi};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
// use embedded_graphics::primitives::PrimitiveStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    // prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyle, Triangle},
};
use mipidsi::options::{
    ColorInversion, ColorOrder, HorizontalRefreshOrder, RefreshOrder, VerticalRefreshOrder,
};
use {defmt_rtt as _, panic_probe as _};

// Provides the Display builder
use mipidsi::{Builder, TestImage};

use crate::touch::Touch;

const DISPLAY_FREQ: u32 = 64_000_000;
const TOUCH_FREQ: u32 = 200_000;

mod touch;
mod waveshare35;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("Hello World!");

    let bl = p.PIN_13;
    let rst = p.PIN_15;
    let display_cs = p.PIN_9;
    let dcx = p.PIN_8;
    let miso = p.PIN_12;
    let mosi = p.PIN_11;
    let clk = p.PIN_10;
    let touch_cs = p.PIN_16;
    //let touch_irq = p.PIN_17;

    // create SPI
    let mut display_config = spi::Config::default();
    display_config.frequency = DISPLAY_FREQ;
    display_config.phase = spi::Phase::CaptureOnSecondTransition;
    display_config.polarity = spi::Polarity::IdleHigh;
    let mut touch_config = spi::Config::default();
    touch_config.frequency = TOUCH_FREQ;
    touch_config.phase = spi::Phase::CaptureOnSecondTransition;
    touch_config.polarity = spi::Polarity::IdleHigh;

    let spi: Spi<'_, _, Blocking> =
        Spi::new_blocking(p.SPI1, clk, mosi, miso, touch_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let display_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(display_cs, Level::High),
        display_config,
    );
    let touch_spi =
        SpiDeviceWithConfig::new(&spi_bus, Output::new(touch_cs, Level::High), touch_config);

    let mut touch = Touch::new(touch_spi);

    let dcx = Output::new(dcx, Level::Low);
    let rst = Output::new(rst, Level::Low);
    // dcx: 0 = command, 1 = data

    // Enable LCD backlight
    let mut bl = Output::new(bl, Level::High);
    bl.set_high();

    // display interface abstraction from SPI and DC
    // let di = SPIDeviceInterface::new(display_spi, dcx);
    let di = waveshare35::SPIDeviceInterface::new(display_spi, dcx);

    // Define the display from the display interface and initialize it
    let mut display = Builder::new(mipidsi::models::ILI9488Rgb565, di)
        .reset_pin(rst)
        .refresh_order(RefreshOrder::new(
            VerticalRefreshOrder::BottomToTop,
            HorizontalRefreshOrder::RightToLeft,
        ))
        .invert_colors(ColorInversion::Inverted)
        .color_order(ColorOrder::Bgr)
        .display_size(480, 320) // w, h
        .init(&mut Delay)
        .unwrap();

    // Make the display all black
    // TestImage::new().draw(&mut display);
    // display.clear(Rgb565::BLACK).unwrap();
    let area: Rectangle = Rectangle::new(Point::new(50, 50), Size::new(50, 50));
    let _ = display.fill_solid(&area, Rgb565::RED);

    // Draw a smiley face with white eyes and a red mouth
    // draw_smiley(&mut display).unwrap();
    let delay = Duration::from_millis(100);

    let mut on = true;
    loop {
        if let Some((_x, _y)) = touch.read() {
            if on {
                bl.set_low();
                on = false;
            } else {
                bl.set_high();
                on = true;
            }
            Timer::after(delay).await;

            //     let style = PrimitiveStyleBuilder::new()
            //         .fill_color(Rgb565::BLUE)
            //         .build();

            // Rectangle::new(Point::new(x - 1, y - 1), Size::new(3, 3))
            //     .into_styled(style)
            //     .draw(&mut display)
            //     .unwrap();
        }
    }
}

fn _draw_smiley<T: DrawTarget<Color = Rgb565>>(display: &mut T) -> Result<(), T::Error> {
    // Draw the left eye as a circle located at (50, 100), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 100), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw the right eye as a circle located at (50, 200), with a diameter of 40, filled with white
    Circle::new(Point::new(50, 200), 40)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::WHITE))
        .draw(display)?;

    // Draw an upside down red triangle to represent a smiling mouth
    Triangle::new(
        Point::new(130, 140),
        Point::new(130, 200),
        Point::new(160, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(display)?;

    // Cover the top part of the mouth with a black triangle so it looks closed instead of open
    Triangle::new(
        Point::new(130, 150),
        Point::new(130, 190),
        Point::new(150, 170),
    )
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLACK))
    .draw(display)?;

    Ok(())
}
