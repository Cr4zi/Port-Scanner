use std::env;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::sync::atomic::{AtomicU16, Ordering, AtomicBool};
use std::net::TcpStream;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const THREADS_COUNT: usize = 50;
const MAX_PORT: u16 = 65535;

macro_rules! info {
    ($msg:expr $(, $($arg:expr),*)?) => {
        println!("[*] {}", format!($msg $(, $($arg),*)?));
    };
}

macro_rules! okay {
    ($msg:expr $(, $($arg:expr),*)?) => {
        println!("[+] {}", format!($msg $(, $($arg),*)?));
    };
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Usage: script [IP]");
        std::process::exit(1);
    }

    let addr: String = (&args[1]).to_string();
    let addr_clone = addr.clone();
    let open_ports = port_scan(addr);

    for _ in 0..50 {
        print!("-");
    }

    print!("\n");

    for port in open_ports.iter() {
        okay!("{}:{} is open", addr_clone, port);
    }
}

fn port_scan(addr: String) -> Vec<u16> {
    let port = Arc::new(AtomicU16::new(0));
    let stop_flag = Arc::new(AtomicBool::new(false));

    let open_ports: Arc<Mutex<Vec<u16>>> = Arc::new(Mutex::new(Vec::new()));
    
    let mut threads = vec![];

    let sock_ipv4 = match Ipv4Addr::from_str(&addr) {
        Ok(ip) => ip,
        Err(_) => {
            eprintln!("Invalid IPv4 address");
            std::process::exit(1);
        }
    };

    for _ in 0..THREADS_COUNT {
        let port_clone = Arc::clone(&port);
        let stop_flag_clone = Arc::clone(&stop_flag);
        let opens_clone = Arc::clone(&open_ports);

        let handle = thread::spawn(move || {
            while !stop_flag_clone.load(Ordering::SeqCst) {
                let cur_port = port_clone.fetch_add(1, Ordering::SeqCst);

                if cur_port > MAX_PORT {
                    stop_flag_clone.store(true, Ordering::SeqCst);
                    break;
                }

                if scan(sock_ipv4, cur_port) {
                    /* append to open ports list */
                    let mut open_ports = opens_clone.lock().unwrap();
                    open_ports.push(cur_port);
                }
            }

        });
        threads.push(handle);
        
    }

    for handle in threads {
        handle.join().unwrap();
    }

    let open_ports = Arc::try_unwrap(open_ports).map_err(|_| "Failed to unwrap open ports").unwrap();
    match open_ports.into_inner() {
        Ok(open_ports) => open_ports,
        Err(_) => {
            eprintln!("Failed to unwrap mutex!");
            std::process::exit(1)
        }
    }

}

fn scan(addr: Ipv4Addr, port: u16) -> bool {
    let sock = SocketAddr::new(IpAddr::V4(addr), port);
    info!("Trying: {:?}", sock); 
    match TcpStream::connect_timeout(&sock, Duration::from_millis(200)) {
        /* port open */
        Ok(stream) => {
            drop(stream);
            true
        }
        /* port closed */
        Err(_) => {
            false
        }
    }
}
