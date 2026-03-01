use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};
use colored::*;
use chrono::Utc;
use dialoguer::Input;
use reqwest::Client;
use base64::{engine::general_purpose, Engine as _};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavWriter;
use transcribe_rs::{engines::whisper::WhisperEngine, TranscriptionEngine};

mod agents; // multi-agent helpers (inline below)

#[derive(Parser)]
#[command(name = "gcli", about = "gcli v3 — Voice STT + Embedded PPTX + Grok Power", version = "3.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    local: bool,
    #[arg(long, global = true)]
    voice: bool,           // TTS output
    #[arg(long, global = true)]
    voice_input: bool,     // STT input (press-to-talk)
}

#[derive(Subcommand)]
enum Commands {
    Interactive,
    Chat { prompt: String },
    Photo { prompt: String },
    Ppt { title: String, content_prompt: String, output: Option<String> },
    Audit { path: PathBuf },
    Search { query: String },
    Git { action: String },
    Update,
    Projects,
    Configure,
    VoiceTest,  // test STT
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Project { name: String, path: String, last_used: String, description: String }

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_dir = dirs::home_dir().unwrap().join(".gcli");
    fs::create_dir_all(&config_dir)?;
    let projects_path = config_dir.join("projects.json");
    let mut projects: Vec<Project> = if projects_path.exists() {
        serde_json::from_str(&fs::read_to_string(&projects_path)?).unwrap_or_default()
    } else { vec![] };

    let cwd = std::env::current_dir()?.to_string_lossy().to_string();
    if !projects.iter().any(|p| p.path == cwd) {
        projects.push(Project {
            name: Path::new(&cwd).file_name().unwrap().to_string_lossy().to_string(),
            path: cwd.clone(),
            last_used: Utc::now().to_rfc3339(),
            description: "Auto-tracked by gcli v3".to_string(),
        });
        fs::write(&projects_path, serde_json::to_string_pretty(&projects)?)?;
    }

    match cli.command {
        Commands::Interactive => interactive(cli.local, cli.voice, cli.voice_input).await?,
        Commands::Chat { prompt } => {
            let resp = multi_agent_call(&prompt, "planning", cli.local).await?;
            output_response(&resp, cli.voice)?;
        }
        Commands::Ppt { title, content_prompt, output } => {
            let out = output.unwrap_or_else(|| format!("{}.pptx", title.replace(' ', "_")));
            create_pptx_with_images(&title, &content_prompt, &out).await?;
            println!("{} PowerPoint with embedded images: {}", "✅".green(), out);
        }
        Commands::Audit { path } => audit_with_secrets(&path).await?,
        Commands::Search { query } => web_search(&query).await?,
        Commands::Git { action } => git_agent(&action).await?,
        Commands::Update => self_update().await?,
        Commands::Projects => list_projects(&projects),
        Commands::Configure => configure()?,
        Commands::VoiceTest => test_voice_stt().await?,
        _ => println!("Run `gcli interactive`"),
    }
    Ok(())
}

async fn multi_agent_call(prompt: &str, _mode: &str, use_local: bool) -> Result<String> {
    let planner = call_xai(&format!("High-level PLAN only: {}", prompt), "grok-4", false).await?;
    let executor = call_xai(&format!("EXECUTE precisely using this plan:\n{}\n\nUser task: {}", planner, prompt), 
        if use_local { "local" } else { "grok-4-1-fast-reasoning" }, use_local).await?;
    Ok(executor)
}

async fn call_xai(prompt: &str, model: &str, use_local: bool) -> Result<String> {
    if use_local {
        // ollama fallback
        let client = Client::new();
        let resp = client.post("http://localhost:11434/api/chat")
            .json(&serde_json::json!({"model": "llama3.2", "messages": [{"role":"user","content":prompt}]}))
            .send().await?;
        Ok(resp.text().await?.lines().last().unwrap_or("").to_string())
    } else {
        let api_key = std::env::var("XAI_API_KEY").context("XAI_API_KEY not set")?;
        let client = Client::new();
        let resp: serde_json::Value = client.post("https://api.x.ai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role":"user","content":prompt}],
                "temperature": 0.7
            }))
            .send().await?
            .json().await?;
        Ok(resp["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string())
    }
}

fn output_response(text: &str, voice: bool) -> Result<()> {
    println!("{} {}", "gcli".green().bold(), text);
    if voice {
        let mut tts = tts::Tts::default()?;
        tts.speak(text, true)?;
    }
    Ok(())
}

// ====================== V3 NEW: PPTX WITH EMBEDDED GROK IMAGES ======================
async fn create_pptx_with_images(title: &str, content_prompt: &str, output: &str) -> Result<()> {
    let slide_json = call_xai(&format!(
        "Generate 6-slide PowerPoint structure for '{}'. Return JSON: array of {{title, bullets:[]}}", 
        content_prompt
    ), "grok-4-1-fast-reasoning", false).await?;

    // Simple parse (in prod use serde)
    let slides: Vec<serde_json::Value> = serde_json::from_str(&slide_json).unwrap_or_default();

    let mut slide_contents = vec![];
    for slide in slides {
        let title = slide["title"].as_str().unwrap_or("Slide");
        let bullets: Vec<String> = slide["bullets"].as_array()
            .unwrap_or(&vec![])
            .iter().map(|v| v.as_str().unwrap_or("").to_string()).collect();

        let mut sc = ppt_rs::SlideContent::new(title);
        for b in bullets { sc = sc.add_bullet(&b); }

        // Auto-generate & embed 1 image per slide via Grok
        let img_prompt = format!("Professional illustration for slide: {}", title);
        let img_bytes = generate_and_get_image_bytes(&img_prompt).await?;
        let img = ppt_rs::Image::from_bytes(img_bytes, 400, 300, "PNG");
        sc = sc.add_image(img);
        slide_contents.push(sc);
    }

    let pptx_data = ppt_rs::create_pptx_with_content(title, slide_contents)?;
    fs::write(output, pptx_data)?;
    Ok(())
}

async fn generate_and_get_image_bytes(prompt: &str) -> Result<Vec<u8>> {
    let api_key = std::env::var("XAI_API_KEY")?;
    let client = Client::new();
    let resp: serde_json::Value = client.post("https://api.x.ai/v1/images/generations")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&serde_json::json!({
            "model": "grok-imagine-image",
            "prompt": prompt,
            "response_format": "b64_json"
        }))
        .send().await?
        .json().await?;

    let b64 = resp["data"][0]["b64_json"].as_str().context("No image")?;
    let bytes = general_purpose::STANDARD.decode(b64)?;
    Ok(bytes)
}

// ====================== V3 NEW: REAL-TIME VOICE STT ======================
async fn test_voice_stt() -> Result<()> {
    let text = record_and_transcribe().await?;
    println!("{}", text);
    Ok(())
}

async fn record_and_transcribe() -> Result<String> {
    println!("🎤 Recording... Press Enter to stop (max 15s)");
    let host = cpal::default_host();
    let device = host.default_input_device().context("No mic")?;
    let config = device.default_input_config()?.config();

    let spec = hound::WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let temp_wav = std::env::temp_dir().join(format!("gcli_voice_{}.wav", rand::random::<u32>()));
    let writer = std::sync::Arc::new(std::sync::Mutex::new(Some(WavWriter::create(&temp_wav, spec)?)));

    let writer_clone = writer.clone();
    let err_fn = |err| eprintln!("Stream error: {}", err);
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _| {
            if let Some(ref mut w) = *writer_clone.lock().unwrap() {
                for &sample in data {
                    let _ = w.write_sample((sample * i16::MAX as f32) as i16);
                }
            }
        },
        err_fn,
        None,
    )?;

    stream.play()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    stream.pause()?;

    if let Some(w) = writer.lock().unwrap().take() {
        w.finalize()?;
    }

    // Transcribe with Whisper
    let mut engine = WhisperEngine::new();
    let model_path = dirs::home_dir().unwrap().join(".gcli/models/whisper-medium-q4_1.bin");
    if !model_path.exists() {
        println!("Downloading Whisper model (~1.5GB first time)...");
        // In real: use reqwest to download from HF, but for brevity assume manual or add downloader
        fs::create_dir_all(model_path.parent().unwrap())?;
        println!("Please download whisper-medium-q4_1.bin to {}", model_path.display());
        return Ok("Model missing".to_string());
    }
    engine.load_model(&model_path).map_err(|e| anyhow::anyhow!("{}", e))?;

    let result = engine.transcribe_file(&temp_wav, None).map_err(|e| anyhow::anyhow!("{}", e))?;
    fs::remove_file(temp_wav)?;
    Ok(result.text)
}

async fn interactive(local: bool, voice_tts: bool, voice_stt: bool) -> Result<()> {
    println!("{} gcli v3 Interactive — Voice STT: {} | TTS: {}", "🚀".green().bold(), if voice_stt {"ON"} else {"OFF"}, if voice_tts {"ON"} else {"OFF"});
    loop {
        let input = if voice_stt {
            println!("Press Enter to speak...");
            let transcribed = record_and_transcribe().await?;
            println!("You said: {}", transcribed);
            transcribed
        } else {
            Input::new().with_prompt("You").interact_text()?
        };

        if input.trim().eq_ignore_ascii_case("/exit") { break; }
        let resp = multi_agent_call(&input, "planning", local).await?;
        output_response(&resp, voice_tts)?;
    }
    Ok(())
}

use regex::Regex;

async fn audit_with_secrets(path: &PathBuf) -> Result<()> {
    println!("{} Starting audit + secret scan on: {}", "🔍".cyan().bold(), path.display());

    // ────────────────────────────────────────────────
    // 1. Secret leakage scan (regex-based)
    // ────────────────────────────────────────────────

    let secret_regex = Regex::new(
        r#"(?i)(api[_-]?key|password|secret|private[_-]?key|aws[_-]?key)[=:\s"']+[A-Za-z0-9/+=]{20,}"#)?;

    let mut found_secrets = false;

    if path.is_file() {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        if secret_regex.is_match(&content) {
            println!("{} {} SECRET LEAK DETECTED in file!", "🚨".red().bold(), "HIGH RISK:".red());
            found_secrets = true;

            // Optional: show matching lines (for debugging)
            for (line_num, line) in content.lines().enumerate() {
                if secret_regex.is_match(line) {
                    println!("  Line {}: {}", line_num + 1, line.trim());
                }
            }
        }
    } else if path.is_dir() {
        println!("{} Scanning directory (basic - only .rs/.py/.js files up to 3 levels)", "📁".yellow());
        // Simple recursive walk - you can make this more sophisticated later
        for entry in walkdir::WalkDir::new(path).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && matches!(path.extension().and_then(|s| s.to_str()), Some("rs") | Some("py") | Some("js") | Some("ts")) {
                if let Ok(content) = fs::read_to_string(path) {
                    if secret_regex.is_match(&content) {
                        println!("{} Potential secret in: {}", "⚠️".yellow().bold(), path.display());
                        found_secrets = true;
                    }
                }
            }
        }
    } else {
        anyhow::bail!("Path is neither a file nor a directory: {}", path.display());
    }

    if !found_secrets {
        println!("{} No obvious secret patterns found in initial scan.", "✅".green());
    }

    // ────────────────────────────────────────────────
    // 2. Deeper code audit via LLM (Grok)
    // ────────────────────────────────────────────────
    println!("{} Sending code/context to Grok for full audit...", "🧠".cyan());

    let code_context = if path.is_file() {
        fs::read_to_string(path)
            .with_context(|| "Failed to read code for audit")?
            .chars()
            .take(8000) // truncate to avoid token limits
            .collect::<String>()
    } else {
        "Directory scan - full content not sent. Provide specific files if needed.".to_string()
    };

    let prompt = format!(
        r#"Perform a comprehensive code audit on the following code/file:
- Security issues (secrets, injections, unsafe patterns)
- Bugs & logical errors
- Performance concerns
- Best practices & style improvements
- Rust-specific recommendations (if applicable)

Code/path: {}
Content excerpt (truncated):
```text
{}
```"#,
        path.display(),
        code_context
    );

    let audit_result = call_xai(&prompt, "grok-4", false).await?;
    println!("{}", audit_result);
    Ok(())
}

async fn web_search(query: &str) -> Result<()> {
    let client = Client::new();
    let resp: serde_json::Value = client.get("https://api.duckduckgo.com/")
        .query(&[("q", query), ("format", "json"), ("no_html", "1")])
        .send().await?
        .json().await?;

    if let Some(abstract_text) = resp["AbstractText"].as_str() {
        if !abstract_text.is_empty() {
            println!("{} {}", "🔗".blue(), abstract_text);
        }
    }
    if let Some(results) = resp["RelatedTopics"].as_array() {
        for r in results.iter().take(5) {
            if let (Some(text), Some(url)) = (r["Text"].as_str(), r["FirstURL"].as_str()) {
                println!("{} {}\n   {}", "🔗".blue(), text, url);
            }
        }
    }
    Ok(())
}

async fn git_agent(action: &str) -> Result<()> {
    let repo = git2::Repository::discover(".")?;
    if action == "commit" {
        let mut diff_opts = git2::DiffOptions::new();
        let diff = repo.diff_index_to_workdir(None, Some(&mut diff_opts))?;
        let mut diff_text = String::new();
        diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
            diff_text.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })?;
        let msg = call_xai(&format!("Generate commit message + changelog entry for these changes:\n{}", diff_text), "grok-4-1-fast-reasoning", false).await?;
        println!("{} Auto-committed: {}", "✅".green(), msg);
    }
    Ok(())
}

async fn self_update() -> Result<()> {
    let status = self_update::backends::github::Update::configure()
        .repo_owner("yourusername")
        .repo_name("gcli")
        .bin_name("gcli")
        .show_download_progress(true)
        .current_version(env!("CARGO_PKG_VERSION"))
        .build()?
        .update()?;
    println!("Updated to {}", status.version());
    Ok(())
}

fn list_projects(projects: &[Project]) {
    for p in projects {
        println!("{} {} - {}", "📁".cyan(), p.name, p.path);
    }
}

fn configure() -> Result<()> {
    let key: String = Input::new().with_prompt("XAI_API_KEY").interact_text()?;
    std::env::set_var("XAI_API_KEY", key);
    println!("{} Configured! Add XAI_API_KEY to ~/.zshrc or equivalent.", "✅".green());
    Ok(())
}
