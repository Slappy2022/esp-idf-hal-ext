use esp_idf_hal::delay::FreeRtos;
use esp_idf_hal::gpio::*;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_hal_ext::sdmmc::Sdmmc;

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().unwrap();
    let mut led = PinDriver::output(peripherals.pins.gpio33)?;

    loop {
        FreeRtos::delay_ms(1000);
        let sdcard = Sdmmc::new("/sdcard")?;
        let info = match sdcard.info() {
            Some(info) => info,
            None => continue,
        };
        println!(
            "{}MB used {}MB total",
            (info.total_bytes - info.free_bytes) / 1_000_000,
            info.total_bytes / 1_000_000,
        );
        {
            // write
            let file = match sdcard.open_file("write.txt", "w") {
                Some(f) => f,
                None => {
                    println!("File not found");
                    continue;
                }
            };
            match file.write(b"write") {
                Ok(()) => println!("write success!"),
                Err(()) => println!("write failure :("),
            }
        }
        {
            // read
            let file = match sdcard.open_file("write.txt", "r") {
                Some(f) => f,
                None => {
                    println!("File not found");
                    continue;
                }
            };
            let data = file.read_vec();
            let data = match core::str::from_utf8(&data) {
                Ok(data) => data,
                Err(e) => {
                    println!("Couldn't parse string: {e:?}");
                    continue;
                }
            };
            match data == "write" {
                true => println!("read success!"),
                false => println!("read failure :("),
            }
        }
        {
            // mkdir/rmdir
            let _ = sdcard.rmdir("mydir");
            if let Err(e) = sdcard.mkdir("mydir") {
                println!("{e:?}");
                continue;
            }
            if let Err(e) = sdcard.rmdir("mydir") {
                println!("{e:?}");
                continue;
            }
            println!("mkdir success");
        }
        {
            // ls root
            let dir = match sdcard.open_directory("") {
                Some(dir) => dir,
                None => {
                    println!("Couldn't open dir");
                    continue;
                }
            };
            for entry in dir.ls() {
                match entry.name() {
                    Ok(name) => println!("{name}"),
                    Err(e) => println!("{e:?}"),
                }
            }
        }
        led.toggle()?;
    }
}
