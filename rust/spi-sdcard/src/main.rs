//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use rp_pico::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::OutputPin;
use panic_probe as _;

use rp_pico::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio,
    pac,
    sio::Sio,
    spi,
    timer,
    watchdog::Watchdog,
};

// Embed the `Hz` function/trait:
use fugit::RateExtU32;

// Link in the embedded_sdmmc crate.
// The `SdMmcSpi` is used for block level access to the card.
// And the `VolumeManager` gives access to the FAT filesystem functions.
use embedded_sdmmc::{SdCard, TimeSource, Timestamp, VolumeIdx, VolumeManager};

// Get the file open mode enum:
use embedded_sdmmc::filesystem::Mode;

use embedded_hal::blocking::delay::DelayMs;
use embedded_hal::blocking::delay::DelayUs;

/// A dummy timesource, which is mostly important for creating files.
#[derive(Default)]
pub struct DummyTimesource();

impl TimeSource for DummyTimesource {
    // In theory you could use the RTC of the rp2040 here, if you had
    // any external time synchronizing device.
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub struct Delayer {
    timer: timer::Timer,
}

impl Delayer {
    pub fn new(timer: timer::Timer) -> Self {
        Delayer { timer }
    }
}

impl DelayMs<u8> for Delayer {
    fn delay_ms(&mut self, ms: u8) {
        self.timer.delay_ms(ms);
    }
}

impl DelayUs<u8> for Delayer {
    fn delay_us(&mut self, us: u8) {
        self.timer.delay_us(us);
    }
}

fn err_loop(t: &mut timer::Timer) -> ! {
    loop {
        error!("error loop");
        t.delay_ms(5000);
    }
}

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut timer = rp_pico::hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let delayer = Delayer::new(timer);

    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead. If you have
    // a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here.
    let mut led_pin = pins.led.into_push_pull_output();

    // Set up our SPI pins into the correct mode
    let spi_sclk: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio2.reconfigure();
    let spi_mosi: gpio::Pin<_, gpio::FunctionSpi, gpio::PullNone> = pins.gpio3.reconfigure();
    let spi_miso: gpio::Pin<_, gpio::FunctionSpi, gpio::PullUp> = pins.gpio4.reconfigure();
    let spi_cs = pins.gpio5.into_push_pull_output();

    // Create the SPI driver instance for the SPI0 device
    let spi = spi::Spi::<_, _, _, 8>::new(pac.SPI0, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        400.kHz(), // card initialization happens at low baud rate
        embedded_hal::spi::MODE_0,
    );

    info!("Initialize SPI SD/MMC data structures...");
    let sdcard = SdCard::new(spi, spi_cs, delayer);
    let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource::default());

    info!("Init SD card controller and retrieve card size...");
    match volume_mgr.device().num_bytes() {
        Ok(size) => info!("card size is {} bytes", size),
        Err(e) => {
            error!("Error retrieving card size: {}", defmt::Debug2Format(&e));
        }
    }

    // Now that the card is initialized, clock can go faster
    volume_mgr
        .device()
        .spi(|spi| spi.set_baudrate(clocks.peripheral_clock.freq(), 16.MHz()));

    info!("Getting Volume 0...");
    let volume = match volume_mgr.open_volume(VolumeIdx(0)) {
        Ok(v) => v,
        Err(e) => {
            error!("Error opening volume 0: {}", defmt::Debug2Format(&e));
            err_loop(&mut timer);
        }
    };

    // After we have the volume (partition) of the drive we got to open the
    // root directory:
    let dir = match volume_mgr.open_root_dir(volume) {
        Ok(dir) => dir,
        Err(e) => {
            error!("Error opening root dir: {}", defmt::Debug2Format(&e));
            err_loop(&mut timer);
        }
    };

    info!("Root directory opened!");

    // This shows how to iterate through the directory and how
    // to get the file names (and print them in hope they are UTF-8 compatible):
    volume_mgr
        .iterate_dir(dir, |ent| {
            info!(
                "/{}.{}",
                core::str::from_utf8(ent.name.base_name()).unwrap(),
                core::str::from_utf8(ent.name.extension()).unwrap()
            );
        })
        .unwrap();

    let mut successful_read = false;

    // Next we going to read a file from the SD card:
    if let Ok(file) = volume_mgr.open_file_in_dir(dir, "O.TST", Mode::ReadOnly) {
        let mut buf = [0u8; 32];
        let read_count = volume_mgr.read(file, &mut buf).unwrap();
        volume_mgr.close_file(file).unwrap();

        if read_count >= 2 {
            info!("READ {} bytes: {}", read_count, buf);

            // If we read what we wrote before the last reset,
            // we set a flag so that the success blinking at the end
            // changes it's pattern.
            if buf[0] == 0x42 && buf[1] == 0x1E {
                successful_read = true;
            }
        }
    }

    match volume_mgr.open_file_in_dir(dir, "DEBUG.TST", Mode::ReadWriteCreateOrTruncate) {
        Ok(file) => {
            volume_mgr
                .write(file, b"\x42\x1E")
                .unwrap();
            volume_mgr.close_file(file).unwrap();
        }
        Err(e) => {
            error!("Error opening file 'O.TST': {}", defmt::Debug2Format(&e));
        }
    }

    match volume_mgr.open_file_in_dir(dir, "TEST_DBG.TXT", Mode::ReadWriteCreateOrTruncate) {
        Ok(file) => {
            let body: &[u8] = b"This is my test text file.\r\nI really hope you like it!!";
            volume_mgr
                .write(file, body)
                .unwrap();
            volume_mgr.close_file(file).unwrap();
        }
        Err(e) => {
            error!("Error opening file 'TEST.TXT': {}", defmt::Debug2Format(&e));
        }
    }

    volume_mgr.free();

    if successful_read {
        info!("Successfully read previously written file 'O.TST'");
    } else {
        info!("Could not read file, which is ok for the first run.");
        info!("Reboot the pico!");
    }

    info!("SD-CARD OK! LET'S GET BLINKY...");
    loop {
        led_pin.set_high().unwrap();
        timer.delay_ms(100);
        led_pin.set_low().unwrap();
        timer.delay_ms(100);
    }
}
