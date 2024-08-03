use console::style;
use homedir::my_home;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

#[derive(Serialize, Deserialize)]
struct Downloads {
    windows: String,
    linux: String,
    macos: String,
}
#[derive(Serialize, Deserialize)]
struct Size {
    windows: u64,
    linux: u64,
    macos: u64,
}
#[derive(Serialize, Deserialize)]
struct VersionResponse {
    version: String,
    name: String,
    changelog: String,
    download: Downloads,
    sizes: Size,
}

#[tokio::main]
async fn main() {
    print!("\x1B[2J\x1B[1;1H");
    println!("Lilith Launcher");
    println!("===============");
    let user_home = my_home().unwrap().unwrap();
    let lilith_dir = user_home.join("lilith");

    if !lilith_dir.try_exists().unwrap() {
        fs::create_dir_all(&lilith_dir).expect("Could not create Lilith data dir!");
    }
    let spin = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template(
            format!(
                "{} » Checking Lilith version {{spinner}}",
                style("Launcher").color256(220)
            )
            .as_str(),
        )
        .unwrap(),
    );
    spin.enable_steady_tick(Duration::from_millis(100));

    let version_request = reqwest::get("http://localhost:8080/versions/latest").await;
    match version_request {
        Ok(version_response) => {
            let version_json: VersionResponse = version_response
                .json::<VersionResponse>()
                .await
                .expect("TODO: panic message");
            spin.finish_and_clear();
            let lilith_file_path =
                lilith_dir.join(version_json.download.macos.rsplit('/').next().unwrap());

            if !lilith_file_path.try_exists().unwrap() {
                println!("{} » A new version of Lilith is available: {}\nThis update brings these changes to Lilith: \n{}", style("Launcher").color256(220), style(&version_json.name).underlined(), &version_json.changelog);

                download_release(version_json, lilith_file_path.clone()).await;
            }
            launch_lilith(lilith_file_path.as_os_str().to_str().unwrap())
        }
        Err(e) => {
            spin.finish_and_clear();
            println!("Could not get the latest version! Error: \n{}", e)
        }
    }
}

async fn download_release(version_response: VersionResponse, lilith_path: PathBuf) {
    let download_link = if cfg!(target_os = "macos") {
        version_response.download.macos
    } else if cfg!(target_os = "windows") {
        version_response.download.windows
    } else {
        version_response.download.linux
    };

    let size = if cfg!(target_os = "macos") {
        version_response.sizes.macos
    } else if cfg!(target_os = "windows") {
        version_response.sizes.windows
    } else {
        version_response.sizes.linux
    };

    let bar = ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template(
            format!(
                "{} » Downloading Lilith v{}, {{msg}} {{spinner}}",
                style("Launcher").color256(220),
                &version_response.version
            )
            .as_str(),
        )
        .unwrap(),
    );
    let mut res = reqwest::Client::new()
        .get(download_link)
        .header(reqwest::header::USER_AGENT, "Lilith Launcher v4")
        .send()
        .await
        .map_err(|_| "Failed to download Lilith, please try again".to_string())
        .unwrap();

    if res.status().is_success() {
        let mut lilith_file = File::create_new(&lilith_path).unwrap();
        let mut downloaded_size: usize = 0;

        while let Some(chunk) = res.chunk().await.unwrap() {
            bar.tick();
            bar.set_message(format!("({}% done)", downloaded_size * 100 / size as usize));
            lilith_file.write_all(&chunk).unwrap();
            downloaded_size += chunk.len();
        }
        bar.finish_and_clear();

        println!(
            "{} » Downloaded Lilith v{}",
            style("Launcher").color256(220),
            version_response.version
        );

        #[cfg(target_os = "macos")]
        Command::new("chmod")
            .arg("+x")
            .arg(&lilith_path)
            .spawn()
            .unwrap_or_else(|_| {
                panic!(
                    "Could not make Lilith an executable. Please run chmod +x {}",
                    lilith_path.as_os_str().to_str().unwrap()
                )
            });
        #[cfg(target_os = "macos")]
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn launch_lilith(lilith_path: &str) {
    match Command::new(lilith_path).arg("--iknowwhatimdoing").status() {
        Ok(_) => {}
        Err(_) => {
            println!(
                "{} » Your Lilith installation is corrupted, relaunching.",
                style("Launcher").color256(220)
            );
            fs::remove_file(lilith_path).expect("Couldn't remove Lilith");

            Command::new(
                std::env::current_exe()
                    .unwrap()
                    .as_os_str()
                    .to_str()
                    .unwrap(),
            )
            .status()
            .expect("Couldn't relaunch the Launcher");
        }
    }
}
