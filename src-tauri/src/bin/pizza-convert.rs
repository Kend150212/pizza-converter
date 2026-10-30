//! Hidden CLI for QA's automated test matrix (`qa-release/test-matrix/run-matrix.py`).
//! Calls the real engine — no GUI, no Tauri.

use convertx_engine as engine;
use std::path::PathBuf;

struct Opts {
    inputs: Vec<PathBuf>,
    output: Option<PathBuf>,
    quality: u8,
    pro: bool,
    merge: bool,
    split_dir: Option<PathBuf>,
    compress: bool,
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("--test-selection") {
        let sel = convertx_lib::selection::current_selection();
        println!("SELECTED_COUNT={}", sel.len());
        for p in sel {
            println!("SELECTED_FILE={}", p.display());
        }
        return;
    }
    if let Err(e) = run(&args) {
        eprintln!("pizza-convert: error: {e}");
        std::process::exit(1);
    }
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        inputs: vec![],
        output: None,
        quality: 90,
        pro: false,
        merge: false,
        split_dir: None,
        compress: false,
    };
    let mut positional: Vec<String> = vec![];
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--quality" => {
                i += 1;
                o.quality = args
                    .get(i)
                    .ok_or("--quality needs a value")?
                    .parse()
                    .map_err(|_| "bad --quality value")?;
            }
            "--pro" => o.pro = true,
            "--merge" => o.merge = true,
            "--compress" => o.compress = true,
            "--split" => {
                i += 1;
                o.split_dir = Some(PathBuf::from(
                    args.get(i).ok_or("--split needs a directory")?,
                ));
            }
            s if s.starts_with("--") => return Err(format!("unknown flag {s}")),
            s => positional.push(s.to_string()),
        }
        i += 1;
    }
    if positional.is_empty() {
        return Err("usage: pizza-convert <input[,..]> <output> [--quality N] [--pro] [--merge] [--split DIR] [--compress]".into());
    }
    o.inputs = positional[0]
        .split(',')
        .map(PathBuf::from)
        .collect();
    if positional.len() > 1 {
        o.output = Some(PathBuf::from(&positional[1]));
    }
    Ok(o)
}

fn run(args: &[String]) -> Result<(), String> {
    let opts = parse(args)?;
    if opts.merge {
        if opts.inputs.len() < 2 {
            return Err("--merge requires at least 2 input files".into());
        }
        let out = opts
            .output
            .ok_or("--merge requires an output path")?;
        engine::merge_pdfs(&opts.inputs, &out, opts.pro).map_err(|e| e.to_string())?;
        println!("OK {}", out.display());
        return Ok(());
    }
    if let Some(dir) = opts.split_dir {
        let input = opts
            .inputs
            .first()
            .ok_or("--split requires 1 input PDF")?;
        let pages = engine::split_pdf(input, &dir, opts.pro).map_err(|e| e.to_string())?;
        println!("OK {} pages -> {}", pages.len(), dir.display());
        return Ok(());
    }
    if opts.compress {
        let input = opts
            .inputs
            .first()
            .ok_or("--compress requires 1 input PDF")?;
        let out = opts
            .output
            .ok_or("--compress requires an output path")?;
        engine::compress_pdf(input, &out, opts.pro).map_err(|e| e.to_string())?;
        println!("OK compressed -> {}", out.display());
        return Ok(());
    }
    let input = opts.inputs.first().ok_or("no input")?;
    let out = opts.output.ok_or("no output")?;
    let _out_ext = out
        .extension()
        .and_then(|s| s.to_str())
        .ok_or("output lacks extension")?;
    engine::convert_image(input, &out, opts.quality, opts.pro)
        .map_err(|e| e.to_string())?;
    println!("OK {}", out.display());
    Ok(())
}
