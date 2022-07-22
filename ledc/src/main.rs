use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

fn main() {
    let scale = 0.0050;
    let max = 0xffff;
    let scaled_max = (scale * max as f64) as i32;
    let mut backwards = false;
    loop {
        for unscaled in 0..scaled_max {
            let us = if backwards { max - unscaled } else { unscaled };
            let x = (us as f64 / scale) as i32;
            let x_msb = (x >> 8) as u8;
            let x_lsb = (x & 0xff) as u8;
            io::stdout()
                .write(&vec![x_msb, x_lsb, x_msb, x_lsb, 0, 0])
                .unwrap();
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        backwards = !backwards;
    }
}
