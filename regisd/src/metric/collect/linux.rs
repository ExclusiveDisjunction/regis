use tokio::{fs::File, io::{BufReader, AsyncBufReadExt}};

use super::prelude::*;
use std::{collections::HashSet, sync::{Arc, OnceLock}};

const MEMORY_TARGETS: &[&str] = &["MemTotal", "MemAvailable", "MemFree","Buffers","Cached"];
const SWAP_TARGETS: &[&str] = &["SwapTotal", "SwapFree", "SwapCached"];
static MEMORY_TARGETS_SET: OnceLock<Arc<HashSet<&'static str>>> = OnceLock::new();
static SWAP_TARGETS_SET: OnceLock<Arc<HashSet<&'static str>>> = OnceLock::new();

fn load_targets(targets: &[&'static str]) -> Arc<HashSet<&'static str>> {
    let mut result: HashSet<&'static str> = HashSet::new();
    for target in targets {
       result.insert(target);
    }

    Arc::new(result)
}
fn load_memory_targets() -> Arc<HashSet<&'static str>> {
    load_targets(MEMORY_TARGETS)
}
fn load_swap_targets() -> Arc<HashSet<&'static str>> {
    load_targets(SWAP_TARGETS)
}

struct Targets {
    mem: Arc<HashSet<&'static str>>,
    swap: Arc<HashSet<&'static str>>
}

fn get_targets() -> Targets {
    let mem = Arc::clone(MEMORY_TARGETS_SET.get_or_init(load_memory_targets));
    let swap = Arc::clone(SWAP_TARGETS_SET.get_or_init(load_swap_targets));

    Targets {
        mem,
        swap
    }
}

fn parse_to_first_whitespace(input: &str) -> &str {
    let mut index = 0usize;
    let mut chars = input.chars();
    while let Some(char) = chars.next() && !char.is_whitespace() {
        index += 1;
    }

    &input[0..index]
}
#[test]
fn test_prefix_parsing() {
    let results = [
        ("abc def", "abc"),
        (" def", ""),
        ("abc", "abc")
    ];

    for (input, result) in results {
        assert_eq!(parse_to_first_whitespace(input), result);
    }
}

pub struct LinuxCollector;
impl MetricsCollector for LinuxCollector {
     async fn cpu() -> Option<CpuMetric> {
        todo!()
     }
     async fn memory() -> Vec<MemoryMetric> {
        let targets = get_targets();

        let mut lines = match File::open("/proc/meminfo").await {
            Ok(v) => BufReader::new(v).lines(),
            Err(_) => return vec![]
        };

        let mut main = MemoryMetric::default();
        let mut swap = MemoryMetric::default();
        while let Ok(line) = lines.next_line().await {
            let line = match line {
                Some(v) => v,
                None => continue
            };

            let prefix = parse_to_first_whitespace(&line);
            if targets.mem.contains(prefix) {
                todo!()
            }
            else if targets.swap.contains(prefix) {
                todo!()
            }
        }
     }
     async fn network() -> Vec<NetworkMetric> {
        todo!()
     }
     async fn storage() -> Vec<StorageMetric> {
        todo!()
     }
}
