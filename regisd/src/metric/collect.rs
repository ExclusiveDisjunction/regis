/*
    Metric collection

    This module is used to handle metric parsing using specific commands.

    Memory
    CPU
    Network
    Disk Usage
*/

use tokio::process::Command;

use common::log_warning;
pub use common::metric::*;

pub async fn collect_memory() -> Option<MemorySnapshot> {
    if cfg!(target_os = "linux") {
        let output = Command::new("free").arg("-b").output().await.ok()?;

        if !output.status.success() {
            return None;
        }

        let raw = String::from_utf8_lossy(&output.stdout).to_string();
        let by_line: Vec<&str> = raw.split("\n")
            .skip(1) //Skip the header
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect();

        /*
           Output pattern
           _        total    used  free   shared    buff    available
           [Name]: [total] [used] [free] [shared] [buff] [availiable]
        */

        if by_line.len() <= 1 {
            return None; //only the header was printed???
        }

        let mut list: Vec<MemoryMetric> = vec![];
        for line in by_line{
            let cols: Vec<&str> = line.split(" ")
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
                .collect();

            //This is a greedy approach. It will attempt to fill as much as possible.
            //The first four are required

            if cols.len() < 4 {
                return None; //Invalid length
            }

            let mut iter = cols.into_iter();
            let mut name = iter.next()?;
            name = match name.strip_suffix(':') {
                Some(n) => n,
                None => name
            };

            let mut converted = iter
                .map(|x| x.parse::<u64>().ok())
                .map(|x| x.map(BinaryNumber::parse));

            list.push(MemoryMetric {
                name: name.to_string(),
                total: converted.next()??,
                used: converted.next()??,
                free: converted.next()??,
                shared: converted.next().unwrap_or(None),
                buff: converted.next().unwrap_or(None),
                available: converted.next().unwrap_or(None),
            })
        }

        Some(MemorySnapshot::new(list))
    } else {
        None
    }
}

pub async fn collect_storage() -> Option<StorageSnapshot> {
    if !cfg!(target_os="linux") {
        return None;
    }

    let raw_output = Command::new("df")
        .arg("-BK")
        .output()
        .await;

    let output = match raw_output {
        Ok(v) => v,
        Err(e) => {
            log_warning!("Unable to collect storage metrics '{e}'");
            return None;
        }
    };

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let by_line: Vec<&str> = raw.split("\n")
        .skip(1) //Skip the header 
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())//Skips empty lines
        .collect(); 

    let mut result: Vec<StorageMetric> = vec![];
    for line in by_line {
        let line = line.trim();
        if !line.starts_with("/dev") {
            continue;
        }

        let splits: Vec<&str> = line.split(' ')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect();

        //Len should be 6
        if splits.len() != 6 {
            continue;
        }

        let name = splits[0].to_owned();
        let disk_stats: Vec<BinaryNumber> = splits[1..4]
            .iter()
            .map(|x| {
                let raw = match x.strip_suffix("K") {
                    Some(v) => v,
                    None => x
                };

                let parsed: u64 = raw.parse().unwrap_or(0) * 1024; //Since the unit is KB, we want it in B.
                BinaryNumber::parse(parsed)
            })
            .collect();

        let mut raw_capacity = splits[4];
        raw_capacity = match raw_capacity.strip_suffix('%') {
            Some(v) => v.trim(),
            None => raw_capacity
        };
        let capacity = Utilization::new(raw_capacity.parse::<u8>().unwrap_or(0)).ok()?;
        let mounted = splits[5].to_owned();

        result.push(
            StorageMetric {
                system: name,
                mount: mounted,
                size: disk_stats[0],
                used: disk_stats[1],
                availiable: disk_stats[2],
                capacity,
            }
        )
    }

    Some(
        StorageSnapshot::new(result)
    )
}

pub async fn collect_cpu() -> Option<CpuMetric> {
    if !cfg!(target_os="linux") {
        return None;
    }
    
    let raw_output = Command::new("sh")
        .args(["-c", "top -b -n 1 | grep \"%Cpu(s)\""])
        .output()
        .await;

    let output = match raw_output {
        Ok(v) => v,
        Err(e) => {
            log_warning!("(Metrics) Unable to collect CPU '{e}'");
            return None;   
        }
    };

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let without_cpu = raw.strip_prefix("%Cpu(s): ")?.trim();
    let comma_splits: Vec<&str> = without_cpu.split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();

    /*  
        Format at this point: 
        [Value] [suffix]

        We need to remove the suffix. It always comes with a space after the value, so we can split by space, and only keep the first one.

        There should be exaclty 8 arguments.
     */
    
    let raw_values: Vec<&str> = comma_splits.into_iter()
        .filter_map(|x| x.split(' ').next()) //Convert it from an option to a real value
        .filter(|x| !x.trim().is_empty())
        .collect(); //Remove the empty entries

    let parsed_values: Vec<u16>= raw_values.into_iter()
        .map(|x| x.parse::<f64>())
        .filter_map(|x| {
            if let Ok(v) = x {
                Some(v as u16)
            }
            else {
                None
            }
        })
        .collect();

    if parsed_values.len() != 8 {
        return None;
    }

    //The first four are supposed to be utiliziations, the remainder are to be interpreted as u16 durations.

    let utils: Vec<Utilization> = parsed_values[0..4]
        .iter()
        .flat_map(|x | Utilization::new(*x as u8))
        .collect();

    if utils.len() != 4 {
        return None;
    }

    Some(
        CpuMetric {
            user: utils[0],
            system: utils[1],
            nice: utils[2],
            idle: utils[3],
            waiting: parsed_values[4],
            h_interupts: parsed_values[5],
            s_interupts: parsed_values[6],
            steal: parsed_values[7],
        }
    )
}

pub async fn collect_network() -> Option<NetworkSnapshot> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let output = Command::new("netstat")
        .arg("-i")
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    /*

        The format is: 

        -Ignore Row-
        -Ignore Row-
        [Name] [Mtu] [RX..4] [TX..4] -Ignore-
     */

    let by_line: Vec<&str> = raw.split('\n')
        .skip(2) //Skip the headers
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();

    let mut result: Vec<NetworkMetric> = vec![];
    for line in by_line {
        let splits: Vec<&str> = line.split(' ')
            .map(|x| x.trim())
            .filter(|x| !x.is_empty())
            .collect();

        if splits.len() != 11 {
            continue;
        }

        let name = splits[0].to_owned();
        let mtu = splits[1].to_owned();
        let rx_raw: Vec<&str> = splits[2..6].to_vec();
        let tx_raw: Vec<&str> = splits[6..10].to_vec();
        //The flag `splits[10]` is ignored

        let rx_values: Vec<u64> = rx_raw.into_iter()
            .map(|x| x.parse::<u64>().unwrap_or(0))
            .collect();
        let tx_values: Vec<u64> = tx_raw.into_iter()
            .map(|x| x.parse::<u64>().unwrap_or(0))
            .collect();

        let rx = NetworkMetricSection::try_from(rx_values).ok()?;
        let tx = NetworkMetricSection::try_from(tx_values).ok()?;

        result.push(
            NetworkMetric {
                name,
                mtu,
                rx,
                tx
            }
        )
    }   

    Some(
        NetworkSnapshot::new(result)
    )
}

pub async fn collect_process_count() -> Option<ProcessCount> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    let raw_output = Command::new("sh")
        .args(["-c", "ps -e --no-headers | wc -l"])
        .output()
        .await;

    let output = match raw_output {
            Ok(v) => v,
            Err(e) => {
                log_warning!("(Collection) Unable to get process count, error '{e}'");
                return None
            }
        };

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let amount: u64 = raw.parse().ok()?;

    Some(
        ProcessCount {
            count: amount
        }
    )
}

pub async fn collect_all_snapshots() -> CollectedMetrics {
    let time = chrono::Local::now().timestamp();

    CollectedMetrics {
        time,
        memory: collect_memory().await,
        storage: collect_storage().await,
        cpu: collect_cpu().await,
        network: collect_network().await,
        proc_count: collect_process_count().await
    }
}