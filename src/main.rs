extern crate reed_solomon;

use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;

use reed_solomon::Decoder;
use reed_solomon::Encoder;

// length of reed solomon error correction code per message
const ECC_LEN: usize = 16;
const CLIENT_OPENVPN_GATEWAY: &str = "127.0.0.1:50272";
const SERVER_TRANSFER_PORT: &str = "127.0.0.1:50273";

fn print_usage(args: Vec<String>) {
    println!(
        "USAGE:\n  {0} server openvpn_ip_port\n  {0} client server_ip_port",
        args[0]
    );
}

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
            if remote != src_addr {
                println!("WARNING: remote {} != {}", remote, src_addr);
            }
            in_tx.send(buf[..num_bytes].to_vec()).unwrap();
        });
    });
    (out_tx, in_rx)
}

fn run_server(openvpn_ip_port: String) {
    println!("Starting Skype VPN server...");
    let upstream_addr: SocketAddr = openvpn_ip_port.parse().unwrap();
    let upstream = UdpSocket::bind("127.0.0.1:0").unwrap();
    upstream.connect(upstream_addr).unwrap();
    println!("Connected to OpenVPN!");
    let (upstream_tx, upstream_rx) = handle_1_1_udp(upstream, Some(upstream_addr));
    let client_sock = UdpSocket::bind(SERVER_TRANSFER_PORT).unwrap();
    println!("Listening for client on {}", SERVER_TRANSFER_PORT);
    let (client_tx, client_rx) = handle_1_1_udp(client_sock, None);

    // forward packets from openvpn server to client
    let t1 = thread::spawn(move || {
        let enc = Encoder::new(ECC_LEN);
        for packet in upstream_rx {
            println!("Got upstream packet: {:?}", packet);
            client_tx.send(enc.encode(&packet).to_vec()).unwrap();
        }
    });

    // forward packets from client to openvpn server
    let t2 = thread::spawn(move || {
        let dec = Decoder::new(ECC_LEN);
        for packet in client_rx {
            println!("Got packet from client: {:?}", packet);
            match dec.correct(&packet[..], None) {
                Ok(decoded) => {
                    println!("Decoded to {:?}", decoded);
                    upstream_tx.send(decoded.data().to_vec()).unwrap();
                }
                Err(_) => {}
            }
        }
    });

    t1.join().unwrap();
    t2.join().unwrap();
}

fn run_client() {
    println!("Starting Skype VPN client...");
    // listen on new port for openvpn client to connect to
    let client_sock = UdpSocket::bind(CLIENT_OPENVPN_GATEWAY).unwrap();

    let socket_addr = SERVER_TRANSFER_PORT.parse::<SocketAddr>().unwrap();
    let server_sock = UdpSocket::bind("127.0.0.1:0").unwrap();
    let (server_tx, server_rx) = handle_1_1_udp(server_sock, Some(socket_addr));
    println!("Connected to server!");

    println!("Listening on 127.0.0.1:50272");
    let (client_tx, client_rx) = handle_1_1_udp(client_sock, None);

    // forward packets from openvpn client to server
    let t1 = thread::spawn(move || {
        let enc = Encoder::new(ECC_LEN);
        for packet in client_rx {
            println!("Got openvpn client packet: {:?}", packet);
            let encoded = enc.encode(&packet);
            println!("Sending {:?}", encoded.to_vec());
            assert!(
                Decoder::new(ECC_LEN)
                    .correct(&encoded.to_vec(), None)
                    .is_ok()
            );
            server_tx.send(encoded.to_vec()).unwrap();
        }
    });

    // forward packets from server to openvpn client
    let t2 = thread::spawn(move || {
        for packet in server_rx {
            println!("Got server packet: {:?}", packet);
            let dec = Decoder::new(ECC_LEN);
            match dec.correct(&packet[..], None) {
                Ok(decoded) => {
                    println!("Decoded: {:?}", decoded);
                    client_tx.send(decoded.data().to_vec()).unwrap();
                }
                Err(_) => {
                    println!("Failed to decode packet!");
                }
            }
        }
    });
    t1.join().unwrap();
    t2.join().unwrap();
}

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() < 2 {
        print_usage(args);
        return;
    }
    match args[1].as_ref() {
        "client" => {
            run_client();
        }
        "server" => {
            if args.len() != 3 {
                print_usage(args);
                return;
            }
            run_server(args[2].clone());
        }
        _ => {
            print_usage(args);
        }
    };
}
