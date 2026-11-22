#![no_std]
#![no_main]
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
//use embassy_rp::pio_programs::spi;
use embassy_rp::uart::{Uart, Config};
use embassy_rp::uart::Blocking as UartBlocking;
use embassy_rp::i2c::{self, I2c, Instance};
use embassy_rp::i2c::Blocking as I2cBlocking;
use embassy_rp::spi::{Spi, Config as SpiConfig};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use mipidsi::{Builder, models::ILI9341Rgb565};
use mipidsi::interface::SpiInterface;
use mipidsi::options::ColorOrder;
use core::fmt::Write;
use embassy_time::Delay;
use embassy_time::Timer;
use embassy_time::{Duration, Instant, Ticker};
use {defmt_rtt as _, panic_probe as _};
use defmt::info;
mod graph;
use graph::ShortGraph;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Emissions Tracker"),
    embassy_rp::binary_info::rp_program_description!(
        c"This is a program to determine air quality and potentialy some chemical output from 3d printing filament offgas"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];

struct UartWriter<'a, 'd>(&'a mut Uart<'d, UartBlocking>);

impl<'a, 'd> core::fmt::Write for UartWriter<'a, 'd> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.blocking_write(s.as_bytes()).map_err(|_| core::fmt::Error)
    }
}

async fn sgp30_init<T: Instance>(sensor: &mut I2c<'_, T, I2cBlocking>) {
    sensor.blocking_write(0x58u16, &[0x20, 0x03]).unwrap();
    Timer::after_millis(10).await;
}

// Measure_IAQ (0x2008): returns CO2-equivalent (ppm) then TVOC (ppb).
// Datasheet max conversion time is 12ms; the SGP30's dynamic baseline
// compensation algorithm requires this to be triggered at exactly 1Hz, so
// the caller is responsible for pacing calls to this, not just the delay below.
async fn sgp30_measure_iaq<T: Instance>(sensor: &mut I2c<'_, T, I2cBlocking>) -> (u16, u16) {
    sensor.blocking_write(0x58u16, &[0x20, 0x08]).unwrap();
    Timer::after_millis(15).await;
    let mut buf = [0u8; 6];
    sensor.blocking_read(0x58u16, &mut buf).unwrap();
    let co2 = u16::from_be_bytes([buf[0], buf[1]]);
    let voc = u16::from_be_bytes([buf[3], buf[4]]);
    (co2, voc)
}

// Measure_raw_signals (0x2050): returns raw H2 signal then raw Ethanol signal.
// Datasheet max conversion time is 25ms; unlike Measure_IAQ this has no
// baseline-compensation timing requirement, so it can be called as fast as
// the conversion time allows.
async fn sgp30_measure_raw<T: Instance>(sensor: &mut I2c<'_, T, I2cBlocking>) -> (u16, u16) {
    sensor.blocking_write(0x58u16, &[0x20, 0x50]).unwrap();
    Timer::after_millis(30).await;
    let mut buf = [0u8; 6];
    sensor.blocking_read(0x58u16, &mut buf).unwrap();
    let h2 = u16::from_be_bytes([buf[0], buf[1]]);
    let ethanol = u16::from_be_bytes([buf[3], buf[4]]);
    (h2, ethanol)
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    
    //i2c sensor releated
    let mut sgp30_sensor = i2c::I2c::new_blocking(p.I2C0, p.PIN_1, p.PIN_0, i2c::Config::default());
    let mut led = Output::new(p.PIN_18, Level::High);
    //let mut backlight_level = Output::new(p.PIN_7,Level::Low);   Was for testing in switching over to pico pi tiny2350

    
    //Uart Related
    let mut uarbest = Uart::new_blocking(p.UART1, p.PIN_26, p.PIN_27, Config::default());
    let mut writer = UartWriter(&mut uarbest);

    
    //SPI related
    let display_clk = p.PIN_2; // done for tiny2350
    let posi_pin = p.PIN_3 ; //serial data in, goes to RX Done for tiny2350
    let mut data_select_pin = Output::new(p.PIN_4, Level::High); // Theoretically done tiny2350
    let mut cont_sig = Output::new(p.PIN_5, Level::High); //done for tiny2350
    let mut reset_pin = Output::new(p.PIN_6,Level::Low);
    let mut backlight_level = Output::new(p.PIN_7,Level::High);
    //let miso_pin = p.PIN_20;
    let mut tft_config = SpiConfig::default();
    tft_config.frequency = 4000000; //32mhz

    let tft_display = Spi::new_blocking_txonly(p.SPI0, display_clk, posi_pin, tft_config);
    
    let spi_dev = ExclusiveDevice::new(tft_display, cont_sig, Delay).unwrap();
    let mut buffer_spi = [0u8;512];
    let di = SpiInterface::new(spi_dev, data_select_pin, &mut buffer_spi);

    write!(writer, "Display init starting\r\n").unwrap();

    let mut display = match Builder::new(ILI9341Rgb565, di)
        .display_size(240, 320)
        .color_order(ColorOrder::Bgr)
        .reset_pin(reset_pin)
        .init(&mut Delay)
    {
        Ok(d) => {
            write!(writer, "Display init OK\r\n").unwrap();
            d
        }
        Err(e) => {
            // Blink the onboard LED fast forever so a failure is visible even
            // without a UART terminal attached, and report the error over UART.
            write!(writer, "Display init FAILED: {:?}\r\n", e).ok();
            loop {
                led.set_high();
                Timer::after_millis(100).await;
                led.set_low();
                Timer::after_millis(100).await;
            }
        }
    };

    display.clear(Rgb565::RED).unwrap();
    backlight_level.set_high();

    sgp30_init(&mut sgp30_sensor).await;
    write!(writer, "Program Start\r\n").unwrap();

    // Display updates are parked for now (see ongoing display debugging) --
    // a single full-screen SPI clear costs hundreds of ms, which alone blows
    // the 50ms budget below many times over.

    const RAW_PERIOD_MS: u64 = 50; // 20Hz, within the SGP30's ~25ms raw conversion limit
    const IAQ_EVERY_N_TICKS: u32 = (1000 / RAW_PERIOD_MS) as u32; // Measure_IAQ stays pinned to 1Hz

    let mut ticker = Ticker::every(Duration::from_millis(RAW_PERIOD_MS));
    let mut tick: u32 = 0;

    loop {
        ticker.next().await;
        led.toggle();

        let (h2, ethanol) = sgp30_measure_raw(&mut sgp30_sensor).await;
        let raw_t = Instant::now().as_millis() as f64 / 1000.0;
        write!(writer, "H2,{:.3},{}\r\n", raw_t, h2).unwrap();
        write!(writer, "ETH,{:.3},{}\r\n", raw_t, ethanol).unwrap();

        if tick % IAQ_EVERY_N_TICKS == 0 {
            let (co2, voc) = sgp30_measure_iaq(&mut sgp30_sensor).await;
            let iaq_t = Instant::now().as_millis() as f64 / 1000.0;
            write!(writer, "CO2,{:.3},{}\r\n", iaq_t, co2).unwrap();
            write!(writer, "TVOC,{:.3},{}\r\n", iaq_t, voc).unwrap();
        }

        tick = tick.wrapping_add(1);
    }
}