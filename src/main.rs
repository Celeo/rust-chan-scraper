use anyhow::{anyhow, Result};
use getopts::{Matches, Options};
use rayon::prelude::*;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::{env, fs, path::Path};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.77 Safari/537.36";

fn get_image_urls(client: &Client, root_url: &str) -> Result<Vec<(String, String)>> {
    let mut files = Vec::new();
    let resp = client.get(root_url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("Got status code {} from URL", resp.status()));
    }
    let body = resp.text()?;
    let html = Html::parse_document(&body);
    let selector = Selector::parse(".fileText a").unwrap();

    for element in html.select(&selector) {
        let href = format!("https:{}", element.value().attr("href").unwrap());
        let name = {
            if let Some(title) = element.value().attr("title") {
                title
            } else {
                element
                    .text()
                    .next()
                    .ok_or_else(|| anyhow!("Could not get element text"))?
            }
        }
        .to_owned();
        files.push((href, name));
    }

    Ok(files)
}

fn download_image(client: &Client, output_dir: &Path, name: &str, url: &str) -> Result<()> {
    let resp = client.get(url).send()?;
    if !resp.status().is_success() {
        return Err(anyhow!("Got status code {} from URL", resp.status()));
    }
    let body = resp.bytes()?;
    let final_out = output_dir.join(name);
    fs::write(&final_out, body)?;
    println!("Downloaded {}", final_out.display());
    Ok(())
}

fn download_page(matches: &Matches) -> Result<()> {
    let url = matches.free[0].clone();
    let output_dir = if let Some(ref s) = matches.opt_default("d", ".") {
        Path::new(s).to_path_buf()
    } else {
        Path::new(".").to_path_buf()
    };
    let client = Client::builder().user_agent(USER_AGENT).build().unwrap();
    println!("Downloading thread {}", url);
    let urls = get_image_urls(&client, &url)?;

    urls.par_iter().for_each(move |(url, name)| {
        if let Err(e) = download_image(&client, &output_dir, name, url) {
            eprintln!("Error downloading file: {}", e);
        }
    });

    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt("d", "directory", "download directory", "./path/to/");
    opts.optflag("h", "help", "print this help menu");

    let matches = opts.parse(&args[1..]).expect("Could not parse CLI args");
    if matches.free.is_empty() {
        eprintln!("Error: must supply URL");
        return;
    }
    if matches.opt_present("h") {
        print!("{}", opts.usage("Usage: rust-chan-scraper [options] URL"));
    }

    match download_page(&matches) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
