#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

mod millis;

use core::panic::PanicInfo;

use arduino_hal::{
    default_serial, delay_ms,
    hal::port::Dynamic,
    pins,
    port::{mode::Output, Pin},
    prelude::_unwrap_infallible_UnwrapInfallible,
    Peripherals,
};
use avr_device::interrupt::disable;
use millis::{millis, init};
use ufmt::uwriteln;

const WIDTH: usize = 5;
const HEIGHT: usize = 3;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    disable();

    let dp = unsafe { Peripherals::steal() };
    let pins = pins!(dp);
    let mut serial = default_serial!(dp, pins, 57600);

    uwriteln!(&mut serial, "Firmware panic!\r").unwrap_infallible();
    if let Some(loc) = info.location() {
        uwriteln!(
            &mut serial,
            "  At {}:{}:{}\r",
            loc.file(),
            loc.line(),
            loc.column(),
        )
        .unwrap_infallible();
    }

    let mut led = pins.d13.into_output();
    loop {
        led.toggle();
        delay_ms(100);
    }
}

#[derive(Clone, Copy, Default)]
struct Frame(u16);

impl Frame {
    const fn new(frame: u16) -> Self {
        Self(frame & !(u16::MAX << (WIDTH * HEIGHT)))
    }
    fn row(self, index: usize) -> [bool; WIDTH] {
        let shifted = self.0 >> ((HEIGHT - 1 - index) * WIDTH);
        let mut row = [false; WIDTH];
        for (index, dot) in row.iter_mut().enumerate() {
            *dot = (shifted >> (WIDTH - 1 - index)) & 1 == 1;
        }
        row
    }
}

struct Display {
    columns: [Pin<Output, Dynamic>; WIDTH],
    rows: [Pin<Output, Dynamic>; HEIGHT],
    frame: Frame,
    row: usize,
}

impl Display {
    fn new(columns: [Pin<Output, Dynamic>; WIDTH], rows: [Pin<Output, Dynamic>; HEIGHT]) -> Self {
        Self {
            columns,
            rows,
            row: 0,
            frame: Frame::default(),
        }
    }
    fn set(&mut self, frame: Frame) {
        self.frame = frame;
    }
    fn update(&mut self) {
        for row in &mut self.rows {
            row.set_high();
        }
        for column in &mut self.columns {
            column.set_high();
        }
        for (index, row) in self.rows.iter_mut().enumerate() {
            if index == self.row {
                row.set_low();
            } else {
                row.set_high();
            }
        }
        for (column, dot) in self.columns.iter_mut().zip(self.frame.row(self.row)) {
            if dot {
                column.set_low();
            }
        }
        self.row += 1;
        self.row %= 3;
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();
    init(&dp.TC0);
    let pins = pins!(dp);
    let mut display = Display::new(
        [
            pins.d8.into_output().downgrade(),
            pins.d9.into_output().downgrade(),
            pins.d10.into_output().downgrade(),
            pins.d11.into_output().downgrade(),
            pins.d12.into_output().downgrade(),
        ],
        [
            pins.d7.into_output_high().downgrade(),
            pins.d6.into_output_high().downgrade(),
            pins.d5.into_output_high().downgrade(),
        ],
    );
    let mut frames = [None; WIDTH * HEIGHT];
    for (index, frame) in frames.iter_mut().enumerate() {
        *frame = Some(Frame::new(1 << index | 1 << (WIDTH * HEIGHT - 1) >> index));
    }
    let frames = frames.map(Option::unwrap);
    loop {
        display.set(frames[(millis() / 200) as usize % frames.len()]);
        display.update();
        delay_ms(5);
    }
}
