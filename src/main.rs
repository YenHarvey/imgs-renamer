use chrono::Local;
use clap::Parser;
use image::{self, io::Reader as ImageReader, ImageFormat};
use rand::{distributions::Alphanumeric, Rng};
use std::fmt::Write;
use std::time::Instant;
use std::{error::Error, fs::create_dir, io, path::Path};
use std::io::prelude::*;
use std::io::Write as _;
use walkdir::WalkDir;
use log::LevelFilter;
use fern;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, default_value = "test", help = "要重命名的文件夹路径")]
    dir: String,

    #[clap(short, long, default_value = "temp", help = "重命名后保存的文件夹路径")]
    to_dir: String,

    #[clap(short, long, default_value = "png", help = "新的文件后缀名")]
    new_postfix: String,

    #[clap(short, long, default_value = "1", help = "图片文件起始索引值")]
    start_index: u32,
}

fn setup_logging() -> Result<(), fern::InitError> {
    if !Path::new("logs").exists() {
        create_dir("logs")?;
    }

    let log_file = Local::now().format("logs/%Y-%m-%d.log").to_string();

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        // .chain(std::io::stdout())
        .chain(fern::log_file(log_file)?)
        .apply()?;

    Ok(())
}

fn is_image_file(file_path: &str) -> bool {
    match image::open(file_path) {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn generate_new_filename(index: u32) -> String {
    let now = Local::now();
    let datetime = now.format("%Y%m%d%H%M%S").to_string();
    let millis = now.timestamp_subsec_millis();
    let random_str: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(5)
        .map(char::from)
        .collect();
    let mut file_name = format!("{}{:03}{}", datetime, millis, random_str);
    write!(file_name, "{:05}", index).unwrap();
    file_name
}

fn convert_image_format(
    input_path: &Path,
    output_path: &Path,
    new_postfix: &str,
) -> Result<(), Box<dyn Error>> {
    let img = ImageReader::open(input_path)?.decode()?;
    match new_postfix {
        "png" => img.save_with_format(output_path, ImageFormat::Png),
        "jpg" | "jpeg" => img.save_with_format(output_path, ImageFormat::Jpeg),
        "webp" => img.save_with_format(output_path, ImageFormat::WebP),
        _ => Err("Unsupported format")?,
    }?;
    Ok(())
}

fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    write!(stdout, "按任意键退出...").unwrap();
    stdout.flush().unwrap();

    let _ = stdin.read(&mut [0u8]).unwrap();
}

fn main() {
    setup_logging().expect("初始化日志配置失败");

    let start_time = Instant::now();
    log::info!("程序开始运行...");

    let args = Args::parse();
    let dir = args.dir;
    let mut to_dir = Path::new(&args.to_dir).to_path_buf();
    if !to_dir.is_absolute() {
        to_dir = std::env::current_dir().unwrap().join(&to_dir);
    }
    if !to_dir.exists() {
        create_dir(&to_dir).unwrap();
    }

    let new_postfix = args.new_postfix;
    let mut start_index = args.start_index;

    println!("开始处理文件夹: {} ...", dir);

    let bar = ProgressBar::new_spinner();
    bar.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}").unwrap().tick_strings(&["-", "\\", "|", "/"]));

    for entry in WalkDir::new(&dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                println!("遍历错误: {}", e);
                log::error!("遍历错误: {}", e);
                continue;
            }
        };

        bar.set_message(format!("正在处理: {}", entry.path().display()));

        if !entry.path().is_dir() && is_image_file(entry.path().to_str().unwrap_or("")) {
            let new_name = format!("{}.{}", generate_new_filename(start_index), &new_postfix);
            let new_path = to_dir.join(&new_name);

            if let Err(err) = convert_image_format(entry.path(), &new_path, &new_postfix) {
                println!("图片格式转换失败: {}", err);
                log::error!("图片格式转换失败: {}", err);
                continue;
            }

            log::info!(
                "重命名并转换格式成功: {} -> {}",
                entry.path().display(),
                new_path.display()
            );
            start_index += 1;
        } else {
            log::warn!("{} 不是一个图片文件或是一个目录!", entry.path().display());
        }
        bar.tick();
    }

    let elapsed = start_time.elapsed();
    bar.finish_with_message("处理完成!");
    println!("程序运行结束，总耗时: {:.2?}", elapsed);
    log::info!("程序运行结束，总耗时: {:?}", elapsed);

    pause();
}
