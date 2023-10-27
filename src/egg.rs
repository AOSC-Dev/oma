use anyhow::{bail, Result};
use colored::{ColoredString, Colorize};
use image::{imageops, DynamicImage, GenericImageView, ImageFormat};
use oma_fetch::checksum::Checksum;
use reqwest::blocking::Client;
use std::{
    io::{self, BufWriter, Write},
    path::Path,
};

const SUM: &str = "0e53c4ad2e3559ad86e8432d6eec32fe3c044e62e2bae9fea539bbeaf62f2345";

fn generate_ascii<W: Write>(
    image: DynamicImage,
    background: Option<String>,
    mut buffer: BufWriter<W>,
) -> io::Result<()> {
    let characters = " .,-~!;:=*&%$@#".chars().collect();
    let (width, height) = image.dimensions();
    let actual_scale = width / 150;

    for y in 0..height {
        for x in 0..width {
            if y % (actual_scale * 2) == 0 && x % actual_scale == 0 {
                let element = get_character(image.get_pixel(x, y), &characters, &background);

                buffer.write_all(element.to_string().as_bytes())?;
            }
        }
        // Add a new line at the end of each row
        if y % (actual_scale * 2) == 0 {
            buffer.write_all("\n".as_bytes())?;
        }
    }

    Ok(())
}

fn get_character(
    pixel: image::Rgba<u8>,
    characters: &Vec<char>,
    background: &Option<String>,
) -> ColoredString {
    let intent = if pixel[3] == 0 {
        0
    } else {
        pixel[0] / 3 + pixel[1] / 3 + pixel[2] / 3
    };

    let ch = characters[(intent / (32 + 7 - (7 + (characters.len() - 7)) as u8)) as usize];

    let ch = String::from(ch);
    let ch = ch.truecolor(pixel[0], pixel[1], pixel[2]);

    match background {
        Some(bg) => ch.on_color(bg.to_string()),
        None => ch,
    }
}

fn output() -> io::Result<BufWriter<Box<dyn Write>>> {
    let output_wrap = Box::new(std::io::stdout().lock());

    Ok(BufWriter::with_capacity(1024, output_wrap))
}

pub fn ailurus() -> Result<()> {
    let p = Path::new("/tmp/266.jpg");
    if !p.exists() {
        let client = Client::builder()
            .user_agent(
                "Mozilla/5.0 (X11; AOSC OS; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/117.0",
            )
            .build()?;

        let image = client
            .get("https://i.pximg.net/img-original/img/2023/02/07/23/01/52/105176083_p2.jpg")
            .header("Referer", "https://www.pixiv.net/")
            .send()?
            .error_for_status()?
            .bytes()?;

        let mut f = std::fs::File::create("/tmp/266.jpg")?;
        let (w, h) = (919, 1157);
        let mut img = image::load_from_memory(&image)?;
        let sub_img = imageops::crop(&mut img, 1200, 60, w, h);
        sub_img.to_image().write_to(&mut f, ImageFormat::Jpeg)?;
    } else if !Checksum::from_sha256_str(SUM)?.cmp_file(p).unwrap_or(false) {
        bail!("你已经有一只小熊猫了！")
    }

    let img = image::open("/tmp/266.jpg")?;
    generate_ascii(img, None, output()?)?;

    Ok(())
}
