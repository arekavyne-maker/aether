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
use std::thread::JoinHandle;

use anyhow::{Context, Result, bail};
use aether_web::{CliRunner, TokenAuth};
use muda::{Menu, MenuEvent, MenuItem, Submenu};
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

/// Headless smoke check: fetch the aggregate over a raw HTTP/1.1 request.
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

fn build_menu() -> Result<Menu> {
    let menu = Menu::new();
    let app = Submenu::new("Aether", true);
    app.append(&MenuItem::new("Quit Aether", true, None))?;
    menu.append(&app)?;
    let view = Submenu::new("View", true);
    view.append(&MenuItem::new("Reload", true, None))?;
    view.append(&MenuItem::new("Toggle theme", true, None))?;
    menu.append(&view)?;
    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();
    Ok(menu)
}

fn main() -> Result<()> {
    configure_bundle_env();
    let (smoke, explicit_project) = parse_args();
    let project = resolve_project(explicit_project)?;
    let server = start_server(project.clone())?;

    if smoke {
        return smoke_check(&server);
    }

    let _menu = build_menu()?;
    let url = format!("http://{}/#token={}", server.addr, server.token);
    let mut allowed = format!("http://{}/", server.addr);
    allowed.truncate(allowed.len() - 1);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(format!("Aether — {}", project.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "local".into())))
        .with_inner_size(tao::dpi::LogicalSize::new(1440, 900))
        .build(&event_loop)?;
    let webview = WebViewBuilder::new()
        .with_url(&url)
        .with_navigation_handler(move |candidate| candidate.starts_with(&allowed))
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
            match menu_event.id.0.as_str() {
                "Quit Aether" => *control_flow = ControlFlow::Exit,
                "Reload" => { let _ = webview.load_url(&url); }
                "Toggle theme" => {
                    let _ = webview.evaluate_script(
                        "document.getElementById('theme-btn')?.click();",
                    );
                }
                _ => {}
            }
        }
        if let tao::event::Event::WindowEvent {
            event: tao::event::WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
