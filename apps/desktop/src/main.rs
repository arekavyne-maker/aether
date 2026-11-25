// SPDX-License-Identifier: LicenseRef-Crocell-Proprietary
// SPDX-FileCopyrightText: 2026 Crocell Company
//! Native Aether shell.
//!
//! The window hosts the same embedded console as `aether-web`. The Axum
//! server runs in-process on an ephemeral loopback port with a per-launch
//! token held only in memory. The canonical CLI is still the only backend.

#![deny(unsafe_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::rc::Rc;
use std::thread::JoinHandle;

use anyhow::{Context, Result, bail};
use aether_web::{CliRunner, TokenAuth};
use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use serde::{Deserialize, Serialize};
use tao::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use tokio::sync::oneshot;
use wry::WebViewBuilder;

struct ServerHandle {
    addr: SocketAddr,
    token: String,
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WindowState {
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
}

fn support_dir() -> Option<PathBuf> {
    let home = std::env::var("HOME").ok()?;
    let dir = PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("Aether");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn read_state<T: for<'de> Deserialize<'de>>(name: &str) -> Option<T> {
    let path = support_dir()?.join(name);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_state<T: Serialize>(name: &str, value: &T) {
    if let Some(path) = support_dir().map(|dir| dir.join(name)) {
        if let Ok(text) = serde_json::to_string_pretty(value) {
            let _ = std::fs::write(path, text);
        }
    }
}

fn recent_projects() -> Vec<String> {
    read_state::<Vec<String>>("recent-projects.json").unwrap_or_default()
}

fn remember_project(path: &PathBuf) {
    let value = path.to_string_lossy().to_string();
    let mut list: Vec<String> = recent_projects()
        .into_iter()
        .filter(|item| item != &value)
        .collect();
    list.insert(0, value);
    list.truncate(6);
    write_state("recent-projects.json", &list);
}

/// Point the canonical CLI at the resources bundled inside the app.
///
/// When running from `target/debug` during development none of these paths
/// exist, so the repository's own discovery rules continue to apply.
fn configure_bundle_env() {
    let Ok(exe) = std::env::current_exe() else { return };
    let Some(macos) = exe.parent() else { return };
    let Some(contents) = macos.parent() else { return };
    let resources = contents.join("Resources");
    if !resources.is_dir() { return; }

    let platform = resources.join("aether");
    if platform.join("contracts").is_dir() {
        std::env::set_var("AETHER_MONOREPO", &platform);
    }
    let cli = resources.join("bin").join("aether");
    if cli.is_file() {
        std::env::set_var("AETHER_BIN", &cli);
    }
    let python = resources.join("python").join("bin").join("python3");
    if python.is_file() {
        std::env::set_var("AETHER_PYTHON", &python);
        let mut paths = vec![platform.join("python")];
        if let Ok(entries) = std::fs::read_dir(resources.join("python").join("lib")) {
            for entry in entries.flatten() {
                let site = entry.path().join("site-packages");
                if site.is_dir() { paths.push(site); }
            }
        }
        let joined = std::env::join_paths(paths)
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();
        std::env::set_var("PYTHONPATH", joined);
    }
    let bin = resources.join("bin");
    if bin.is_dir() {
        let existing = std::env::var("PATH").unwrap_or_default();
        let separator = if cfg!(windows) { ";" } else { ":" };
        let path = format!("{}{}{}", bin.display(), separator, existing);
        std::env::set_var("PATH", path);
    }
}

fn parse_args() -> (bool, Option<PathBuf>) {
    let mut smoke = false;
    let mut project = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke-test" => smoke = true,
            "--project" => project = args.next().map(PathBuf::from),
            other => eprintln!("ignoring unknown argument: {other}"),
        }
    }
    (smoke, project)
}

fn resolve_project(explicit: Option<PathBuf>) -> Result<PathBuf> {
    let candidate = explicit
        .or_else(|| std::env::var("AETHER_WEB_PROJECT").ok().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = candidate.canonicalize().with_context(|| format!("project root {} does not exist", candidate.display()))?;
    if !root.is_dir() {
        bail!("project root {} is not a directory", root.display());
    }
    Ok(root)
}

/// Start the console server in-process and return its discovered address.
fn start_server(project: PathBuf) -> Result<ServerHandle> {
    let listener = TcpListener::bind("127.0.0.1:0").context("cannot bind loopback listener")?;
    let addr = listener.local_addr()?;
    listener.set_nonblocking(true)?;
    let token: TokenAuth = aether_web::auth::generate_token()?;
    let token_hex = token.as_str();
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let thread = std::thread::spawn(move || -> Result<()> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("cannot build server runtime")?;
        runtime.block_on(async move {
            let listener = tokio::net::TcpListener::from_std(listener)?;
            let runner = CliRunner::discover()?.with_project(project);
            let server = aether_web::serve(listener, token, runner);
            tokio::select! {
                result = server => result?,
                _ = shutdown_rx => {}
            }
            Ok(())
        })
    });
    Ok(ServerHandle {
        addr,
        token: token_hex,
        shutdown: Some(shutdown_tx),
        thread: Some(thread),
    })
}

/// Headless smoke check: fetch the atlas over a raw HTTP/1.1 request.
fn smoke_check(server: &ServerHandle) -> Result<()> {
    let mut stream = TcpStream::connect(server.addr)?;
    let request = format!(
        "GET /api/atlas HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n",
        server.addr, server.token
    );
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    if !response.starts_with("HTTP/1.1 200") {
        bail!("atlas request failed: {}", response.lines().next().unwrap_or(""));
    }
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("");
    if !body.contains("\"node_types\"") || !body.contains("\"sources\"") {
        bail!("atlas response is incomplete: {}", &body[..body.len().min(200)]);
    }
    println!("{body}");
    Ok(())
}

fn accel(key: Code) -> Option<Accelerator> {
    Some(Accelerator::new(Modifiers::META, key))
}

fn build_menu(project: &PathBuf) -> Result<Menu> {
    let menu = Menu::new();

    let app = Submenu::with_id("app", "Aether", true);
    app.append(&MenuItem::with_id("about", "About Aether", true, None))?;
    app.append(&MenuItem::with_id("diagnostics", "Diagnostics", true, None))?;
    app.append(&PredefinedMenuItem::separator())?;
    app.append(&MenuItem::with_id("reload", "Reload", true, accel(Code::KeyR)))?;
    app.append(&MenuItem::with_id("theme", "Toggle theme", true, accel(Code::KeyT)))?;
    app.append(&PredefinedMenuItem::separator())?;
    app.append(&MenuItem::with_id("quit", "Quit Aether", true, accel(Code::KeyQ)))?;
    menu.append(&app)?;

    let file = Submenu::with_id("file", "File", true);
    file.append(&MenuItem::with_id("open-project", "Open Project…", true, accel(Code::KeyO)))?;
    file.append(&MenuItem::with_id("show-project", "Current Project", true, None))?;
    let recent = Submenu::with_id("recent", "Recent Projects", true);
    let projects = recent_projects();
    if projects.is_empty() {
        recent.append(&MenuItem::with_id("recent:none", "No recent projects", false, None))?;
    } else {
        for (index, path) in projects.iter().take(6).enumerate() {
            recent.append(&MenuItem::with_id(
                format!("recent:{index}"),
                path.clone(),
                true,
                None,
            ))?;
        }
    }
    file.append(&recent)?;
    menu.append(&file)?;

    let run = Submenu::with_id("run", "Run", true);
    run.append(&MenuItem::with_id("run-command", "Run Command", true, accel(Code::Enter)))?;
    run.append(&MenuItem::with_id("cancel-job", "Cancel Job", true, accel(Code::Period)))?;
    run.append(&PredefinedMenuItem::separator())?;
    run.append(&MenuItem::with_id("terminal", "CLI Terminal", true, accel(Code::KeyJ)))?;
    run.append(&MenuItem::with_id("ledger", "Session Ledger", true, None))?;
    menu.append(&run)?;

    let view = Submenu::with_id("view", "View", true);
    for (id, label) in [
        ("home", "Home"), ("library", "Run"), ("atlas", "Atlas"),
        ("verify", "Verify"), ("commands", "Command Index"),
    ] {
        view.append(&MenuItem::with_id(format!("view:{id}"), label, true, None))?;
    }
    menu.append(&view)?;

    let help = Submenu::with_id("help", "Help", true);
    help.append(&MenuItem::with_id("shortcuts", "Keyboard Shortcuts", true, accel(Code::Slash)))?;
    help.append(&MenuItem::with_id("project-label", project.to_string_lossy().to_string(), false, None))?;
    menu.append(&help)?;

    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();
    Ok(menu)
}

fn navigate(webview: &wry::WebView, route: &str) {
    let script = format!("location.hash='#/{route}'");
    let _ = webview.evaluate_script(&script);
}

fn main() -> Result<()> {
    configure_bundle_env();
    let (smoke, explicit_project) = parse_args();
    let project = resolve_project(explicit_project)?;
    let server = start_server(project.clone())?;

    if smoke {
        return smoke_check(&server);
    }

    remember_project(&project);
    let _menu = build_menu(&project)?;
    let url = format!("http://{}/#token={}", server.addr, server.token);
    let allowed = format!("http://{}/", server.addr);
    let allowed = allowed.trim_end_matches('/').to_string();

    let saved: WindowState = read_state("window.json").unwrap_or_default();
    let event_loop = EventLoop::new();
    let window = Rc::new(
        WindowBuilder::new()
            .with_title(format!("Aether — {}", project.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "local".into())))
            .with_inner_size(LogicalSize::new(
                if saved.width > 0 { saved.width } else { 1440 },
                if saved.height > 0 { saved.height } else { 900 },
            ))
            .build(&event_loop)?,
    );
    if let (Some(x), Some(y)) = (saved.x, saved.y) {
        window.set_outer_position(PhysicalPosition::new(x, y));
    }
    let webview = Rc::new(
        WebViewBuilder::new()
            .with_url(&url)
            .with_navigation_handler(move |candidate| candidate.starts_with(&allowed))
            .build(&window)?,
    );

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            match menu_event.id.0.as_str() {
                "about" => { let _ = webview.evaluate_script("document.getElementById('help-overlay')?.classList.add('open');"); }
                "diagnostics" => navigate(&webview, "build"),
                "reload" => { let _ = webview.load_url(&url); }
                "theme" => { let _ = webview.evaluate_script("document.getElementById('theme-btn')?.click();"); }
                "quit" => *control_flow = ControlFlow::Exit,
                "open-project" | "show-project" => navigate(&webview, "projects"),
                id if id.starts_with("recent:") => {
                    if let Some(index) = id.strip_prefix("recent:").and_then(|value| value.parse::<usize>().ok()) {
                        if let Some(path) = recent_projects().get(index) {
                            let script = format!(
                                "(async()=>{{const r=await fetch('/api/project',{{method:'POST',headers:{{'Authorization':'Bearer '+(localStorage.getItem('aether.token')||'')}} ,body:JSON.stringify({{path:{}}})}});if(r.ok)location.hash='#/pulse';}})()",
                                serde_json::to_string(path).unwrap_or_default()
                            );
                            let _ = webview.evaluate_script(&script);
                        }
                    }
                }
                "run-command" => { let _ = webview.evaluate_script("document.getElementById('cmd-run')?.click();"); }
                "cancel-job" => { let _ = webview.evaluate_script("document.querySelector('[data-cancel]')?.click();"); }
                "terminal" => navigate(&webview, "terminal"),
                "ledger" => navigate(&webview, "session"),
                "view:home" => navigate(&webview, "pulse"),
                "view:library" => navigate(&webview, "run"),
                "view:atlas" => navigate(&webview, "atlas/graph"),
                "view:verify" => navigate(&webview, "assurance"),
                "view:commands" => navigate(&webview, "commands"),
                "shortcuts" => { let _ = webview.evaluate_script("document.getElementById('help-overlay')?.classList.add('open');"); }
                _ => {}
            }
        }
        if let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            ..
        } = event
        {
            let size: PhysicalSize<u32> = window.inner_size();
            let position: Option<PhysicalPosition<i32>> = window.outer_position().ok();
            write_state("window.json", &WindowState {
                width: size.width,
                height: size.height,
                x: position.map(|p| p.x),
                y: position.map(|p| p.y),
            });
            *control_flow = ControlFlow::Exit;
        }
    });
}
