# desk-fcobs - the lights on my desk

*There is an ESP32.*

*There is a [FCOB CCT LED strip](https://www.aliexpress.com/item/1005001614814078.html?spm=a2g0o.order_list.0.0.17001802qIvvjv).*

*There is a [power supply](https://www.amazon.com/Chengliang-Supplys-Original-Printer-Switching/dp/B08QCDQLPY).*

*There is a driver.*

*There is a computer.*

*They are connected.*

There is light.

<TODO: add picture>

## Okay, okay, stop with the romanticization

Fine.

It's just a tiny little firmware that takes in `PPPPWWWWCWCW`, controlled by a little rust driver that currently just fades. I run it then stop it when I'm happy with the setting.

It doesn't manage it's own serial connection yet, so you'll have to `cargo run | picocom /dev/ttyUSB1 -qb 115200`.

## You should automate that!

Yes, yes, I should. Maybe I'll make it work like [`redshift`](http://jonls.dk/redshift/).

## License

This is free and unencumbered software released into the public domain.
See `LICENSE` for more details.
