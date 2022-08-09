use std::{
    io::{self, Write},
    thread,
    time::Duration,
};

#[derive(Eq, PartialEq, Copy, Clone)]
enum Channel {
    Warm,
    Cold,
}

impl Channel {
    fn inv(self) -> Self {
        match self {
            Channel::Cold => Channel::Warm,
            Channel::Warm => Channel::Cold
        }
    }
}

fn fade(scaled_max: i32, max: i32, scale: f64, backwards: bool, ch: Channel) {
    for unscaled in 0..scaled_max {
        let us = if backwards { max - unscaled } else { unscaled };
        let x = (us as f64 / scale) as i32;
        let msb = (x >> 8) as u8;
        let lsb = ((x & 0xff) as u8).saturating_add(15);
        let mut buf = vec![msb, lsb];
        if ch == Channel::Warm {
            buf.append(&mut vec![msb, lsb, 0, 0]);
        } else {
            buf.append(&mut vec![0, 0, msb, lsb]);
        }
        io::stdout().write(&buf).unwrap();
        io::stdout().flush().unwrap();
        thread::sleep(Duration::from_millis(5));
    }
}

fn main() {
    let scale = 0.002;
    let max = 0xffff;
    let scaled_max = (scale * max as f64) as i32;
    let mut backwards = false;
    let mut channel = Channel::Warm;
    loop {
        fade(scaled_max, max, scale, backwards, channel);
        backwards = !backwards;
        fade(scaled_max, max, scale, backwards, channel);
        backwards = !backwards;
        channel = channel.inv();
    }
}
