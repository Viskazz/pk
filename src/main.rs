use argh::FromArgs;
use std::{
    io::{self, Error, ErrorKind, Write},
    process::{Command, Stdio},
};

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

#[derive(FromArgs)]
#[argh(description = "Please, specify port to kill app")]

struct Args {
    #[argh(option, short = 'p', description = "specify port")]
    port: u16,
}

pub struct Win;
pub struct Unix;

pub trait Killer {
    fn get_pid(&self, port: u16) -> Result<Vec<u32>, Error>;
    fn kill(&self, pid: Vec<u32>) -> Result<bool, Error>;
}

impl Killer for Win {
    fn get_pid(&self, port: u16) -> Result<Vec<u32>, Error> {
        let process = Command::new("powershell")
            .args(&[
                "-Command",
                "netstat",
                "-ano",
                "|",
                "findStr",
                &format!(":{}", port)[..],
            ])
            .output()
            .expect("Failed to execute powershell");
        if process.status.success() {
            let res = String::from_utf8(process.stdout).expect("Failed to convert string");
            let mut res_vec: Vec<u32> = res
                .split(LINE_ENDING)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    s.split_whitespace()
                        .last()
                        .expect("Fail to split")
                        .parse::<u32>()
                        .expect("Failed to parse output")
                })
                .collect();
            // remove dup values
            if res_vec.len() > 1 {
                res_vec.sort_unstable();
                res_vec.dedup();
            }
            Ok(res_vec)
        } else {
            Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                String::from_utf8(process.stderr).unwrap(),
            ))
        }
    }
    fn kill(&self, pid: Vec<u32>) -> Result<bool, Error> {
        let pid_str = pid.iter().map(|&p| p.to_string()).collect::<Vec<String>>();
        let output = Command::new("taskkill")
            .arg("/F")
            .arg("/PID")
            .args(&pid_str)
            .output()
            .expect("Failed to execute process");
        Ok(output.status.success())
    }
}

impl Killer for Unix {
    fn get_pid(&self, port: u16) -> Result<Vec<u32>, io::Error> {
        let lsof_stdout = Command::new("lsof")
            .arg(format!("-i:{}", port))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn lsof, may be you should install `lsof` first")
            .wait_with_output()
            .expect("Failed to wait for lsof")
            .stdout;
        let mut awk_child = Command::new("awk")
            .arg("NR > 1 { print $2 }")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        awk_child
            .stdin
            .as_mut()
            .expect("Fail to get awk stdin")
            .write_all(&lsof_stdout[..])
            .expect("Failed to write to awk stdin");
        let awk_output = awk_child
            .wait_with_output()
            .expect("Failed to wait for awk");
        if awk_output.status.success() {
            let res_str = String::from_utf8(awk_output.stdout).expect("Failed to parse awk output");
            let mut res_vec: Vec<u32> = res_str
                .split(LINE_ENDING)
                .filter_map(|s| s.parse::<u32>().ok())
                .collect();
            // remove dup values
            if res_vec.len() > 1 {
                res_vec.sort_unstable();
                res_vec.dedup();
            }
            Ok(res_vec)
        } else {
            Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                String::from_utf8(awk_output.stderr).unwrap(),
            ))
        }
    }
    fn kill(&self, pid: Vec<u32>) -> Result<bool, Error> {
        let pid_str = pid.iter().map(|&p| p.to_string()).collect::<Vec<String>>();
        let output = Command::new("kill")
            .arg("-9")
            .args(&pid_str)
            .output()
            .expect("failed to execute process");
        Ok(output.status.success())
    }
}

fn main() {
    let args: Args = argh::from_env();

    let port = args.port;

    match kill_port(port) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => eprintln!("Something wrong :( \n  {}", e),
    };
}

pub fn kill_port(port: u16) -> Result<bool, Error> {
    let killer = if cfg!(target_os = "windows") {
        Box::new(Win) as Box<dyn Killer>
    } else {
        Box::new(Unix) as Box<dyn Killer>
    };
    let pids = killer
        .get_pid(port)
        .unwrap_or_else(|e| panic!("Failed to get pids on port: {}, error: {}", port, e));
    Ok(killer
        .kill(pids)
        .unwrap_or_else(|e| panic!("Failed to kill process on port: {}, error: {}", port, e)))
}
