#![cfg(all(feature = "dist-client", feature = "dist-server"))]

extern crate assert_cmd;
#[macro_use]
extern crate log;
extern crate rstash;
extern crate serde_json;

use crate::harness::{
    cargo_command, get_stats, init_cargo, rstash_command, start_local_daemon, stop_local_daemon,
    write_json_cfg, write_source,
};
use assert_cmd::prelude::*;
use rstash::config::HTTPUrl;
use rstash::dist::{
    AssignJobResult, CompileCommand, InputsReader, JobId, JobState, RunJobResult, ServerIncoming,
    ServerOutgoing, SubmitToolchainResult, Toolchain, ToolchainReader,
};
use std::ffi::OsStr;
use std::path::Path;
use std::process::Output;

use rstash::errors::*;

mod harness;

fn basic_compile(tmpdir: &Path, rstash_cfg_path: &Path, rstash_cached_cfg_path: &Path) {
    let envs: Vec<(_, &OsStr)> = vec![
        ("RUST_BACKTRACE", "1".as_ref()),
        ("RSTASH_LOG", "debug".as_ref()),
        ("RSTASH_CONF", rstash_cfg_path.as_ref()),
        ("RSTASH_CACHED_CONF", rstash_cached_cfg_path.as_ref()),
    ];
    let source_file = "x.c";
    let obj_file = "x.o";
    write_source(tmpdir, source_file, "#if !defined(RSTASH_TEST_DEFINE)\n#error RSTASH_TEST_DEFINE is not defined\n#endif\nint x() { return 5; }");
    rstash_command()
        .args([
            std::env::var("CC")
                .unwrap_or_else(|_| "gcc".to_string())
                .as_str(),
            "-c",
            "-DRSTASH_TEST_DEFINE",
        ])
        .arg(tmpdir.join(source_file))
        .arg("-o")
        .arg(tmpdir.join(obj_file))
        .envs(envs)
        .assert()
        .success();
}

fn rust_compile(tmpdir: &Path, rstash_cfg_path: &Path, rstash_cached_cfg_path: &Path) -> Output {
    let rstash_path = assert_cmd::cargo::cargo_bin("rstash").into_os_string();
    let envs: Vec<(_, &OsStr)> = vec![
        ("RUSTC_WRAPPER", rstash_path.as_ref()),
        ("CARGO_TARGET_DIR", "target".as_ref()),
        ("RUST_BACKTRACE", "1".as_ref()),
        ("RSTASH_LOG", "debug".as_ref()),
        ("RSTASH_CONF", rstash_cfg_path.as_ref()),
        ("RSTASH_CACHED_CONF", rstash_cached_cfg_path.as_ref()),
    ];
    let cargo_name = "rstash-dist-test";
    let cargo_path = init_cargo(tmpdir, cargo_name);

    let manifest_file = "Cargo.toml";
    let source_file = "src/main.rs";

    write_source(
        &cargo_path,
        manifest_file,
        r#"[package]
        name = "rstash-dist-test"
        version = "0.1.0"
        edition = "2021"
        [dependencies]
        libc = "0.2.169""#,
    );
    write_source(
        &cargo_path,
        source_file,
        r#"fn main() {
        println!("Hello, world!");
}"#,
    );

    cargo_command()
        .current_dir(cargo_path)
        .args(["build", "--release"])
        .envs(envs)
        .output()
        .unwrap()
}

pub fn dist_test_rstash_client_cfg(
    tmpdir: &Path,
    scheduler_url: HTTPUrl,
) -> rstash::config::FileConfig {
    let mut rstash_cfg = harness::rstash_client_cfg(tmpdir, false);
    rstash_cfg.cache.disk.as_mut().unwrap().size = 0;
    rstash_cfg.dist.scheduler_url = Some(scheduler_url);
    rstash_cfg
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_basic() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    system.add_server();

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    get_stats(|info| {
        assert_eq!(1, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(0, info.stats.dist_errors);
        assert_eq!(1, info.stats.compile_requests);
        assert_eq!(1, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(1, info.stats.cache_misses.all());
    });
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_restartedserver() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    let server_handle = system.add_server();

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    system.restart_server(&server_handle);
    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    get_stats(|info| {
        assert_eq!(2, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(0, info.stats.dist_errors);
        assert_eq!(2, info.stats.compile_requests);
        assert_eq!(2, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(2, info.stats.cache_misses.all());
    });
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_nobuilder() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    get_stats(|info| {
        assert_eq!(0, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(1, info.stats.dist_errors);
        assert_eq!(1, info.stats.compile_requests);
        assert_eq!(1, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(1, info.stats.cache_misses.all());
    });
}

struct FailingServer;
impl ServerIncoming for FailingServer {
    fn handle_assign_job(&self, _job_id: JobId, _tc: Toolchain) -> Result<AssignJobResult> {
        let need_toolchain = false;
        let state = JobState::Ready;
        Ok(AssignJobResult {
            need_toolchain,
            state,
        })
    }
    fn handle_submit_toolchain(
        &self,
        _requester: &dyn ServerOutgoing,
        _job_id: JobId,
        _tc_rdr: ToolchainReader,
    ) -> Result<SubmitToolchainResult> {
        panic!("should not have submitted toolchain")
    }
    fn handle_run_job(
        &self,
        requester: &dyn ServerOutgoing,
        job_id: JobId,
        _command: CompileCommand,
        _outputs: Vec<String>,
        _inputs_rdr: InputsReader,
    ) -> Result<RunJobResult> {
        requester
            .do_update_job_state(job_id, JobState::Started)
            .context("Updating job state failed")?;
        bail!("internal build failure")
    }
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_failingserver() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    system.add_custom_server(FailingServer);

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    get_stats(|info| {
        assert_eq!(0, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(1, info.stats.dist_errors);
        assert_eq!(1, info.stats.compile_requests);
        assert_eq!(1, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(1, info.stats.cache_misses.all());
    });
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_cargo_build() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    let _server_handle = system.add_server();

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    rust_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path)
        .assert()
        .success();
    get_stats(|info| {
        assert_eq!(1, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(0, info.stats.dist_errors);
        assert_eq!(5, info.stats.compile_requests);
        assert_eq!(1, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(1, info.stats.cache_misses.all());
    });
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_cargo_makeflags() {
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    let _server_handle = system.add_server();

    let rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);
    let compile_output = rust_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    assert!(!String::from_utf8_lossy(&compile_output.stderr)
        .contains("warning: failed to connect to jobserver from environment variable"));

    get_stats(|info| {
        assert_eq!(1, info.stats.dist_compiles.values().sum::<usize>());
        assert_eq!(0, info.stats.dist_errors);
        assert_eq!(5, info.stats.compile_requests);
        assert_eq!(1, info.stats.requests_executed);
        assert_eq!(0, info.stats.cache_hits.all());
        assert_eq!(1, info.stats.cache_misses.all());
    });
}

#[test]
#[cfg_attr(not(feature = "dist-tests"), ignore)]
fn test_dist_preprocesspr_cache_bug_2173() {
    // Bug 2173: preprocessor cache hit but main cache miss - because using the preprocessor cache
    // means not doing regular preprocessing, there was no preprocessed translation unit to send
    // out for distributed compilation, so an empty u8 array was compiled - which "worked", but
    // the object file *was* the result of compiling an empty file.
    let tmpdir = tempfile::Builder::new()
        .prefix("rstash_dist_test")
        .tempdir()
        .unwrap();
    let tmpdir = tmpdir.path();
    let rstash_dist = harness::rstash_dist_path();

    let mut system = harness::DistSystem::new(&rstash_dist, tmpdir);
    system.add_scheduler();
    let _server_handle = system.add_server();

    let mut rstash_cfg = dist_test_rstash_client_cfg(tmpdir, system.scheduler_url());
    let disk_cache = rstash_cfg.cache.disk.as_mut().unwrap();
    disk_cache
        .preprocessor_cache_mode
        .use_preprocessor_cache_mode = true;
    disk_cache.size = 10_000_000; // enough for one tiny object file
    let rstash_cfg_path = tmpdir.join("rstash-cfg.json");
    write_json_cfg(tmpdir, "rstash-cfg.json", &rstash_cfg);
    let rstash_cached_cfg_path = tmpdir.join("rstash-cached-cfg");

    stop_local_daemon();
    start_local_daemon(&rstash_cfg_path, &rstash_cached_cfg_path);

    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);
    let obj_file = "x.o";
    let obj_path = tmpdir.join(obj_file);
    let data_a = std::fs::read(&obj_path).unwrap();

    let cache_path = rstash_cfg.cache.disk.unwrap().dir;

    // Don't touch the preprocessor cache - and check that it exists
    assert!(
        cache_path.join("preprocessor").is_dir(),
        "The preprocessor cache should exist"
    );

    // Delete the main cache to ensure a cache miss - potential dirs are "0".."f".
    let main_cache_dirs = "0123456789abcdef";
    let delete_count = main_cache_dirs.chars().fold(0, |res, dir| {
        res + (std::fs::remove_dir_all(cache_path.join(String::from(dir))).is_ok() as u32)
    });
    assert_eq!(delete_count, 1, "Did the disk cache format change?");

    basic_compile(tmpdir, &rstash_cfg_path, &rstash_cached_cfg_path);

    // Check that this gave the same result (i.e. that it didn't compile a completely empty file).
    // It would be nice to check directly that the object file contains the symbol for the x() function
    // from basic_compile(), but that seems pretty involved and this happens to work...
    let data_b = std::fs::read(&obj_path).unwrap();

    assert_eq!(data_a, data_b);
}
