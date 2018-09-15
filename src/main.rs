use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;

// simple, stupid wrapper around a udp port
// send data in, get data out. to/from only one person.
fn handle_1_1_udp(
    sock: UdpSocket,
    remote_opt: Option<SocketAddr>,
) -> (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) {
    let mut buf = [0; 64 * 1024];
    let (in_tx, in_rx) = mpsc::channel::<Vec<u8>>();
    let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let remote = remote_opt.unwrap_or_else(|| {
            // wait until we get our first packet
            let (num_bytes, client_addr) = sock.recv_from(&mut buf).expect("Didn't receive data");
            in_tx.send(buf[..num_bytes].to_vec()).unwrap();
            client_addr
        });
        sock.connect(remote).unwrap();

        // reply thread
        let sock1 = sock.try_clone().unwrap();
        thread::spawn(move || {
            for to_send in out_rx {
                sock1.send(&to_send).unwrap();
            }
        });

        // handle rest of packets
        thread::spawn(move || loop {
            let (num_bytes, src_addr) = sock.recv_from(&mut buf).expect("Didn't receive data");
            println!("Got packet from {}", src_addr);
            if remote != src_addr {
                println!("WARNING: remote {} != {}", remote, src_addr);
            }
            in_tx.send(buf[..num_bytes].to_vec()).unwrap();
        });
    });
    (out_tx, in_rx)
}

fn run_openvpn_proxy(ip_and_port: String) {
    // listen on new port for openvpn client to connect to
    let client_sock = UdpSocket::bind("127.0.0.1:50272").unwrap();
    println!("Listening on 127.0.0.1:50272");
    let (client_tx, client_rx) = handle_1_1_udp(client_sock, None);

    // connect to openvpn server
    let upstream_addr: SocketAddr = ip_and_port.parse().unwrap();
    let upstream = UdpSocket::bind("127.0.0.1:0").unwrap();
    upstream.connect(upstream_addr).unwrap();
    let (upstream_tx, upstream_rx) = handle_1_1_udp(upstream, Some(upstream_addr));
    let t1 = thread::spawn(move || {
        for packet in client_rx {
            println!("Forwarding client packet: {:?}", packet);
            upstream_tx.send(packet).unwrap();
        }
    });
    let t2 = thread::spawn(move || {
        for packet in upstream_rx {
            println!("Forwarding upstream packet: {:?}", packet);
            client_tx.send(packet).unwrap();
        }
    });
    t1.join().unwrap();
    t2.join().unwrap();
}

fn main() {
    println!("Hello, world!");
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        println!("USAGE: {} openvpn_server_ip:port", args[0]);
    }
    run_openvpn_proxy(args[1].clone());
}
