#![no_std]
#![no_main]
use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;
use embassy_rp::uart::{Uart, Config};
use embassy_rp::uart::Blocking as UartBlocking;
use embassy_rp::i2c::{self, I2c, Instance};
use embassy_rp::i2c::Blocking as I2cBlocking;
use core::fmt::Write;
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

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

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut sgp30_sensor = i2c::I2c::new_blocking(p.I2C0, p.PIN_5, p.PIN_4, i2c::Config::default());
    let mut uarbest = Uart::new_blocking(p.UART0, p.PIN_0, p.PIN_1, Config::default());
    let mut writer = UartWriter(&mut uarbest);
    let mut read_buf = [0u8; 6];

    sgp30_init(&mut sgp30_sensor).await;
    write!(writer, "Program Start\r\n").unwrap();

    loop {
        sgp30_sensor.blocking_write(0x58u16, &[0x20, 0x08]).unwrap();
        Timer::after_millis(12).await;
        sgp30_sensor.blocking_read(0x58u16, &mut read_buf).unwrap();

        let co2 = u16::from_be_bytes([read_buf[0], read_buf[1]]);
        let voc = u16::from_be_bytes([read_buf[3], read_buf[4]]);

        Timer::after_millis(988).await;

        write!(writer, "Co2 reading: {}\r\n", co2).unwrap();
        write!(writer, "VoC reading: {}\r\n", voc).unwrap();
    }
}