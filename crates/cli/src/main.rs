use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal;
use ffi_bridge::{register_native_rust, NativeTag, NativeValue};
use hot_reload::{
    apply_light_reload, build_reload_plan, parse_program_text, start_file_watcher, ReloadPlan,
    WatchEvent,
};
use std::env;
use std::ffi::CStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant, SystemTime};
use vm_core::{Program, VM};

const DEFAULT_SOURCE: &str = "apps/demo/main.svm";

fn main() {
    register_default_native_selectors();

    let args = env::args().skip(1).collect::<Vec<_>>();
    let run_once = args.iter().any(|a| a == "--once");
    let quiet_ticks = args.iter().any(|a| a == "--quiet-ticks");
    let source_path = args
        .iter()
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOURCE));

    if !source_path.exists() {
        eprintln!("source file not found: {}", source_path.display());
        std::process::exit(1);
    }

    let canonical_source = source_path
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize {}: {e}", source_path.display()))
        .unwrap_or_else(|err| {
            eprintln!("{err}");
            std::process::exit(1);
        });

    let mut runtime = match Runtime::new(&canonical_source, quiet_ticks) {
        Ok(rt) => rt,
        Err(err) => {
            eprintln!("failed to start runtime: {err}");
            std::process::exit(1);
        }
    };

    let result = if run_once {
        runtime.run_once()
    } else {
        runtime.run()
    };

    if let Err(err) = result {
        eprintln!("runtime error: {err}");
        std::process::exit(1);
    }
}

fn register_default_native_selectors() {
    let _ = register_native_rust("foundation.string.uppercased", native_string_uppercased);
    let _ = register_native_rust("foundation.math.add2", native_math_add2);
    let _ = register_native_rust("dev.log", native_dev_log);
    let _ = register_native_rust("String", native_string_cast); // Add String conversion
}

extern "C" fn native_string_cast(
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32 {
    if args.is_null() || out_result.is_null() || arg_count < 1 {
        return -9;
    }
    let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
    let val_str = match slice[0].tag {
        NativeTag::Int => slice[0].int_value.to_string(),
        NativeTag::Bool => (slice[0].bool_value != 0).to_string(),
        NativeTag::String if !slice[0].string_ptr.is_null() => {
            unsafe { CStr::from_ptr(slice[0].string_ptr) }.to_string_lossy().into_owned()
        }
        _ => "nil".to_string(),
    };
    
    unsafe {
        *out_result = NativeValue::string(&val_str);
    }
    0
}

extern "C" fn native_string_uppercased(
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32 {
    if args.is_null() || out_result.is_null() || arg_count < 1 {
        return -9;
    }
    let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
    if slice[0].tag != NativeTag::String || slice[0].string_ptr.is_null() {
        return -8;
    }
    let upper = unsafe { CStr::from_ptr(slice[0].string_ptr) }
        .to_string_lossy()
        .to_uppercase();
    unsafe {
        *out_result = NativeValue::string(&upper);
    }
    0
}

extern "C" fn native_math_add2(
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32 {
    if args.is_null() || out_result.is_null() || arg_count < 2 {
        return -9;
    }
    let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
    if slice[0].tag != NativeTag::Int || slice[1].tag != NativeTag::Int {
        return -8;
    }
    unsafe {
        *out_result = NativeValue::int(slice[0].int_value + slice[1].int_value);
    }
    0
}

extern "C" fn native_dev_log(
    args: *const NativeValue,
    arg_count: usize,
    out_result: *mut NativeValue,
) -> i32 {
    if args.is_null() || out_result.is_null() || arg_count < 1 {
        return -9;
    }
    let slice = unsafe { std::slice::from_raw_parts(args, arg_count) };
    match slice[0].tag {
        NativeTag::String if !slice[0].string_ptr.is_null() => {
            let msg = unsafe { CStr::from_ptr(slice[0].string_ptr) }.to_string_lossy();
            println!("[native] {msg}");
        }
        NativeTag::Int => println!("[native] {}", slice[0].int_value),
        NativeTag::Bool => println!("[native] {}", slice[0].bool_value != 0),
        NativeTag::Nil => println!("[native] nil"),
        _ => return -8,
    }
    unsafe {
        *out_result = NativeValue::nil();
    }
    0
}

struct Runtime {
    source_path: PathBuf,
    vm: VM,
    program: Program,
    watch_rx: std::sync::mpsc::Receiver<WatchEvent>,
    _watcher: notify::RecommendedWatcher,
    auto_reload: bool,
    quiet_ticks: bool,
    raw_mode: bool,
    last_source_modified: Option<SystemTime>,
    last_poll_check: Instant,
}

impl Runtime {
    fn new(source_path: &Path, quiet_ticks: bool) -> Result<Self, String> {
        let program = compile_source_to_program(source_path)?;

        let mut vm = VM::new();
        vm.initialize_program_state(&program);
        if program.functions.contains_key("init") {
            vm.run_function(&program, "init")
                .map_err(|e| format!("init failed: {e}"))?;
        }

        let (watch_tx, watch_rx) = channel();
        let parent = source_path
            .parent()
            .ok_or_else(|| "source path must have parent directory".to_string())?;
        let watcher = start_file_watcher(parent, watch_tx).map_err(|e| e.to_string())?;

        Ok(Self {
            source_path: source_path.to_path_buf(),
            vm,
            program,
            watch_rx,
            _watcher: watcher,
            auto_reload: true,
            quiet_ticks,
            raw_mode: false,
            last_source_modified: source_modified_time(source_path),
            last_poll_check: Instant::now(),
        })
    }

    fn log_line(&self, line: impl AsRef<str>) {
        let line = line.as_ref();
        if self.raw_mode {
            let mut out = io::stdout();
            let _ = out.write_all(line.as_bytes());
            let _ = out.write_all(b"\r\n");
            let _ = out.flush();
        } else {
            println!("{line}");
        }
    }

    fn run(&mut self) -> Result<(), String> {
        terminal::enable_raw_mode().map_err(|e| e.to_string())?;
        self.raw_mode = true;

        self.log_line("");
        self.log_line("SwiftVM Dev Server");
        self.log_line(format!("source: {}", self.source_path.display()));
        self.log_line("commands:");
        self.log_line("  r - light reload (preserve state)");
        self.log_line("  R - hard reload (reset state)");
        self.log_line("  a - toggle auto-reload");
        self.log_line("  q - quit");
        self.log_line("");

        let mut next_tick = Instant::now();

        loop {
            while let Ok(event) = self.watch_rx.try_recv() {
                let WatchEvent::SourceChanged(path) = event;
                let canonical_match = path
                    .canonicalize()
                    .map(|p| p == self.source_path)
                    .unwrap_or(false);
                let file_name_match = path.file_name() == self.source_path.file_name();
                if (canonical_match || file_name_match) && self.auto_reload {
                    self.light_reload("file save")?;
                }
            }

            if self.auto_reload && self.last_poll_check.elapsed() >= Duration::from_millis(200) {
                self.last_poll_check = Instant::now();
                let current_modified = source_modified_time(&self.source_path);
                if current_modified.is_some() && current_modified != self.last_source_modified {
                    self.light_reload("file save (poll)")?;
                }
            }

            if event::poll(Duration::from_millis(20)).map_err(|e| e.to_string())? {
                if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('r') => self.light_reload("manual")?,
                            KeyCode::Char('R') => self.hard_reload("manual")?,
                            KeyCode::Char('a') => {
                                self.auto_reload = !self.auto_reload;
                                self.log_line(format!(
                                    "[auto] auto-reload {}",
                                    if self.auto_reload { "ON" } else { "OFF" }
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }

            if Instant::now() >= next_tick {
                // Execute main if it exists, but primarily we want the state
                let _ = self.vm.run_function(&self.program, "main");
                
                // Bridge: Export entire global state as JSON for the iOS app
                let mut state_map = std::collections::HashMap::new();
                for (name, value) in self.vm.globals() {
                    state_map.insert(name, format!("{}", value));
                }
                
                if let Ok(json) = serde_json::to_string(&state_map) {
                    let _ = fs::write("/tmp/swiftvm-state.json", json);
                }
                
                next_tick = Instant::now() + Duration::from_millis(500);
            }
        }

        terminal::disable_raw_mode().map_err(|e| e.to_string())?;
        self.raw_mode = false;
        println!("\nbye");
        Ok(())
    }

    fn run_once(&mut self) -> Result<(), String> {
        let value = self
            .vm
            .run_function(&self.program, "main")
            .map_err(|e| format!("main failed: {e}"))?;
        println!("{value}");
        Ok(())
    }

    fn light_reload(&mut self, source: &str) -> Result<(), String> {
        let new_program = compile_source_to_program(&self.source_path)?;
        let plan = build_reload_plan(&self.program, &new_program);

        match plan {
            ReloadPlan::NoChanges => {
                self.log_line(format!("[reload] {source}: no changes"));
            }
            ReloadPlan::Light {
                changed_functions,
                state_migration,
            } => {
                let changed_names = changed_functions
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                
                // CRITICAL: Update live globals if their default values changed in source
                // We do this BEFORE apply_light_reload so we can compare old vs new
                for (name, new_field) in &new_program.state_layout {
                    if let Some(old_field) = self.program.state_layout.get(name) {
                        if old_field.default_value != new_field.default_value {
                            if let Some(val) = &new_field.default_value {
                                self.vm.update_global(name, val.clone());
                            }
                        }
                    }
                }

                apply_light_reload(&mut self.program, &new_program, &changed_functions);
                self.vm.initialize_program_state(&self.program);
                
                if changed_functions.is_empty() {
                    self.log_line(format!(
                        "[reload] {source}: light reload complete, updated state values"
                    ));
                } else {
                    self.log_line(format!(
                        "[reload] {source}: light reload complete, swapped {} function(s): {}",
                        changed_functions.len(),
                        changed_names
                    ));
                }
                if !state_migration.added_with_defaults.is_empty() {
                    self.log_line(format!(
                        "[reload] {source}: initialized new state fields with defaults: {}",
                        state_migration.added_with_defaults.join(", ")
                    ));
                }
            }
            ReloadPlan::Hard { reason } => {
                self.log_line(format!(
                    "[reload] {source}: hard reload required ({reason}) - press R"
                ));
            }
        }

        self.last_source_modified = source_modified_time(&self.source_path);
        Ok(())
    }

    fn hard_reload(&mut self, source: &str) -> Result<(), String> {
        let new_program = compile_source_to_program(&self.source_path)?;

        self.program = new_program;
        self.vm.reset_runtime_state();
        self.vm.initialize_program_state(&self.program);

        if self.program.functions.contains_key("init") {
            self.vm
                .run_function(&self.program, "init")
                .map_err(|e| format!("init failed after hard reload: {e}"))?;
        }

        self.last_source_modified = source_modified_time(&self.source_path);
        self.log_line(format!("[reload] {source}: hard reload complete, state reset"));
        Ok(())
    }
}

fn read_source(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))
}

fn source_modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

fn compile_source_to_program(source_path: &Path) -> Result<Program, String> {
    match source_path.extension().and_then(|e| e.to_str()) {
        Some("svm") => {
            let source = read_source(source_path)?;
            parse_program_text(&source).map_err(|e| format!("parse error: {e}"))
        }
        Some("swift") => {
            let svm_text = compile_swift_to_svm(source_path)?;
            parse_program_text(&svm_text).map_err(|e| format!("swift frontend parse error: {e}"))
        }
        other => Err(format!(
            "unsupported source extension {:?}, expected .svm or .swift",
            other
        )),
    }
}

fn compile_swift_to_svm(source_path: &Path) -> Result<String, String> {
    let package_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../swift/Frontend")
        .canonicalize()
        .map_err(|e| format!("failed to find swift frontend package: {e}"))?;
    let frontend_bin = package_path.join(".build/debug/swiftvm-frontend");

    if !frontend_bin.exists() {
        let build = Command::new("xcrun")
            .arg("swift")
            .arg("build")
            .arg("--package-path")
            .arg(&package_path)
            .output()
            .map_err(|e| format!("failed to build swift frontend: {e}"))?;

        if !build.status.success() {
            let stderr = String::from_utf8_lossy(&build.stderr);
            return Err(format!("swift frontend build failed: {}", stderr.trim()));
        }
    }

    let output = Command::new(&frontend_bin)
        .arg(source_path)
        .output()
        .map_err(|e| format!("failed to run compiled swift frontend: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("swift frontend failed: {}", stderr.trim()));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("swift frontend output was not valid UTF-8: {e}"))?;
    Ok(stdout)
}
