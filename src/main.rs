use rustube::{Id, VideoFetcher};
use std::process::Command;
use std::{thread, time::Duration};
use structopt::StructOpt;
use thirtyfour::prelude::*;
use tokio::fs;

// !!!! dont forget to set your chromedriver in your system envirenment path !!!
// then run command :
// cargo run -- -u <playlist_url> -s <starting_song> -n <number_of_songs>
// or : cargo run --release -- -u <playlist_url> -s <starting_song> -n <number_of_songs>
#[tokio::main]
async fn main() -> WebDriverResult<()> {
    // Chromedriver command
    Command::new("chromedriver")
        .spawn()
        .expect("chromedriver not found!");

    // Collect args
    let options = Options::from_args();
    let playlist_url = options.url;
    let number_of_scrolls = options.num / 100 + 1;

    // Selenium driver
    let caps = DesiredCapabilities::chrome();
    let driver = WebDriver::new("http://localhost:9515", caps).await?;

    // Navigate to playlist
    driver.goto(playlist_url).await?;

    let mut links = Vec::new();
    let mut skips = Vec::new();

    for _ in 0..number_of_scrolls {
        driver
            .execute("window.scrollBy(0, 12000)", Vec::new())
            .await
            .unwrap();
        thread::sleep(Duration::new(1, 0));
    }
    let thumbnails = driver.find_all(By::Id("thumbnail")).await?;

    for thumbnail in thumbnails {
        match thumbnail.attr("href").await? {
            Some(link) => {
                links.push(format!("www.youtube.com{}", link));
            }
            _ => (),
        }
    }
    println!("found {} songs.", links.len());

    // Always explicitly close the browser
    driver.quit().await?;

    println!("Beginning to download...");

    fs::create_dir_all("downloads").await?;

    for i in (options.start - 1)..options.num {
        //download part
        println!("downloading song n° {}:", i + 1);
        match dl(links[i].clone()).await {
            Ok(some) => some,
            _ => {
                skips.push(&links[i]);
                println!("skip {}", links[i]);
            }
        }
    }

    println!("-------------------------- Songs skipped --------------------------");

    for skip in skips {
        println!("{}", skip);
    }
    Ok(())
}

#[derive(StructOpt)]
struct Options {
    #[structopt(
        short = "u",
        long = "playlist_url",
        default_value = "https://www.youtube.com/playlist?list=PLacfgxju84VisWNxjg13q5og-TA9uKTjA"
    )]
    url: String,
    #[structopt(short = "n", long = "number_of_songs", default_value = "444")]
    num: usize,
    #[structopt(short = "s", long = "starting_song", default_value = "1")]
    start: usize,
}

async fn dl(url: String) -> Result<(), rustube::Error> {
    let id = Id::from_raw(&url)?;
    let descrambler = VideoFetcher::from_id(id.into_owned())
        .unwrap()
        .fetch()
        .await?;

    let video = descrambler.descramble()?;
    let best_quality = video
        .streams()
        .iter()
        .filter(|stream| !stream.includes_video_track && stream.includes_audio_track)
        .max_by_key(|stream| stream.quality_label);
    best_quality.unwrap().download_to_dir("downloads").await?;

    let file_path = format!("downloads/{}.webm", video.id());
    let new_file_path = format!(
        "downloads/{}.mp3",
        video
            .title()
            .replace(&['/', '\\', '*', '<', '>', ':', '?', '"', '|'], " ")
    );
    fs::rename(&file_path, &new_file_path).await?;

    println!("finished downloading {}.", video.title());
    Ok(())
}
