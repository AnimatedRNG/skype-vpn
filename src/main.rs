extern crate reed_solomon;
extern crate crc;
extern crate byteorder;

use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc;
use std::thread;
use std::collections::VecDeque;

use reed_solomon::Decoder;
use crc::crc32;

use byteorder::{ByteOrder, BigEndian};

const MAX_UDP_PACKET_LEN: usize = 576;
const PACKET_LEN_ENCODE_BYTES: usize = 2;
const REED_SOLOMON_BLOCK_LEN: usize = 10;
// length of reed solomon error correction code per block
const ECC_LEN: usize = 8;
const CLIENT_OPENVPN_GATEWAY: &str = "127.0.0.1:50272";
const SERVER_TRANSFER_PORT: &str = "127.0.0.1:50273";
const FRAME_LEN: usize = 1024 * 10;
const HASH_LENGTH_BYTES : usize = 2;
const SEQNO_LENGTH_BYTES : usize = 8;

type Frame = [u8; FRAME_LEN];

fn vec_to_frame(v: Vec<u8>) -> Frame {
    let mut f = [0u8; FRAME_LEN];
    for i in 0..v.len() {
        f[i] = v[i];
    }
    f
}

fn divide_round_up(n: usize, m: usize) -> usize {
    n / m + (n % m != 0) as usize
}

fn predict_rs_size(num_bytes: usize) -> usize {
    let num_blocks = divide_round_up(num_bytes, REED_SOLOMON_BLOCK_LEN);
    num_blocks * (REED_SOLOMON_BLOCK_LEN + ECC_LEN)
}

struct FrameEncoder {
    packets: VecDeque<Vec<u8>>,
    seqno: u64,
}

impl FrameEncoder {
    fn new() -> Self {
        FrameEncoder {packets: VecDeque::new(), seqno: 0}
    }

    fn add_packet(&mut self, packet: Vec<u8>) {
        assert!(packet.len() <= FRAME_LEN);
        self.packets.push_back(packet);
    }

    fn get_next_frame(&mut self) -> Frame {
        let mut predicted: usize = HASH_LENGTH_BYTES + SEQNO_LENGTH_BYTES;
        let mut packet_batch = Vec::new();
        while !self.packets.is_empty() {
            let packet = self.packets.pop_front().unwrap();
            let packet_bytes = packet.len() + PACKET_LEN_ENCODE_BYTES;
            if predict_rs_size(predicted + packet_bytes) < FRAME_LEN {
                packet_batch.push(packet);
                predicted += packet_bytes;
            } else {
                self.packets.push_front(packet);
                break;
            }
        }

        let mut seqno_bytes = vec![0u8; 8];
        BigEndian::write_u64(&mut seqno_bytes, self.seqno);
        println!("seqno bytes: {:?}", seqno_bytes);
        self.seqno += 1;
        // insert seqno at start
        let mut raw_frame_data : Vec<u8> = std::iter::once(seqno_bytes).chain(
            packet_batch.into_iter()
                // prepend packet lengths before each
                .flat_map(|p| {
                    let mut v = vec![0u8; 2];
                    BigEndian::write_u16(&mut v, p.len() as u16);
                    vec![v, p].into_iter()
                })
            )
            // turn into raw bytes
            .flatten()
            .collect();
        let enc = reed_solomon::Encoder::new(ECC_LEN);
        // add crc32 hash to the end of the frame
        let mut v = [0u8; 4];
        BigEndian::write_u32(&mut v, crc32::checksum_ieee(&raw_frame_data));
        raw_frame_data.extend([0u8; 2].into_iter().chain(v.into_iter()));
        // reed solomon it all
        let mut frame : Frame = [0u8; FRAME_LEN];
        println!("rfd: {:?}", raw_frame_data);
        raw_frame_data
            .chunks(REED_SOLOMON_BLOCK_LEN)
            .map(|c| {
                let mut padded = [0u8; REED_SOLOMON_BLOCK_LEN];
                for i in 0..c.len() {
                    padded[i] = c[i];
                }
                padded
            })
            .map(|chunk| {
                println!("{:?}", chunk);
                chunk
            })
            .map(|chunk| enc.encode(&chunk).to_vec())
            .map(|chunk| {
                println!("{:?}", chunk);
                chunk
            })
            .flatten()
            .enumerate()
            .for_each(|(i, p)| frame[i] = p);
         frame
    }
}

struct FrameDecoder {
    next_seqno: u64,
}

impl FrameDecoder {
    fn new() -> Self {
        FrameDecoder {next_seqno: 0}
    }
    fn read_frame(&mut self, f: Frame) -> Option<Vec<Vec<u8>>> {
        if let Some((seqno, packets)) = decode_frame(f) {
            if seqno >= self.next_seqno {
                self.next_seqno = seqno + 1;
                Some(packets)
            } else {
                None
            }
        } else {
            None
        }
    }
}

// returns (seqno, vec of decoded packets)
fn decode_frame(f: Frame) -> Option<(u64, Vec<Vec<u8>>)> {
    let dec = Decoder::new(ECC_LEN);
    let raw_frame_data_opt = f
        .chunks(REED_SOLOMON_BLOCK_LEN + ECC_LEN)
        .take_while(|&c| {
            c.iter()
                .find(|&&x| x != 0)
                .is_some()
        })
        .map(|c| {
            println!("{:?}", c);
            c
        })
        .map(|c| dec.correct(c, None).and_then(
            |buffer| Ok(buffer.data().to_vec())).ok())
        .collect::<Option<Vec<Vec<u8>>>>();
    if raw_frame_data_opt.is_none() {
        println!("Too many errors: unable to read frame");
        return None;
    }
    let raw_frame_data = raw_frame_data_opt.unwrap().into_iter()
        .flatten()
        .collect::<Vec<u8>>();
    let mut last_nonzero_index = raw_frame_data.len()-1;
    while raw_frame_data[last_nonzero_index] == 0 {
        last_nonzero_index -= 1;
    }
    let frame_end = last_nonzero_index + 1;
    println!("rfd(decode): {:?}", raw_frame_data[0..frame_end].to_vec());
    // check hash
    let read_hash = BigEndian::read_u32(&raw_frame_data[frame_end-4..]);
    let calculated_hash = crc32::checksum_ieee(&raw_frame_data[..frame_end-4]);
    if read_hash != calculated_hash {
        println!("Hashes differ: {} != {}", read_hash, calculated_hash);
    }
    println!("seqno bytes: {:?}", &raw_frame_data[0..8]);
    let seqno = BigEndian::read_u64(&raw_frame_data[0..8]);
    let mut packet_offset: usize = 8;
    let mut packets = vec![];
    loop {
        if packet_offset >= frame_end - 2 {
            break;
        }
        let len = BigEndian::read_u16(&raw_frame_data[packet_offset..packet_offset+2]);
        if len == 0 {
            break;
        }
        let packet = raw_frame_data[packet_offset+2..packet_offset+2+len as usize].to_vec();
        packets.push(packet);
        packet_offset += 2 + len as usize
    }
    Some((seqno, packets))
}

#[test]
fn test_frame() {
    let packets = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let mut enc = FrameEncoder::new();
    enc.add_packet(packets[0].clone());
    enc.add_packet(packets[1].clone());
    let f = enc.get_next_frame();
    let (seqno, decoded_packets) = decode_frame(f).unwrap();
    assert_eq!(seqno, 0);
    assert_eq!(packets, decoded_packets);
}

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
    let mut buf = [0; MAX_UDP_PACKET_LEN];
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
        let mut enc = FrameEncoder::new();
        for packet in upstream_rx {
            println!("Got upstream packet: {:?}", packet);
            enc.add_packet(packet);
            client_tx.send(enc.get_next_frame().to_vec()).unwrap();
        }
    });

    // forward packets from client to openvpn server
    let t2 = thread::spawn(move || {
        let mut dec = FrameDecoder::new();
        for packet in client_rx {
            println!("Got packet from client: {:?}", packet);
            if let Some(decoded) = dec.read_frame(vec_to_frame(packet)) {
                for pkt in decoded {
                    println!("Decoded to {:?}", pkt);
                    upstream_tx.send(pkt).unwrap();
                }
            } else {
                println!("Failed to decode packet");
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
        let mut enc = FrameEncoder::new();
        for packet in client_rx {
            println!("Got openvpn client packet: {:?}", packet);
            enc.add_packet(packet);
            server_tx.send(enc.get_next_frame().to_vec()).unwrap();
        }
    });

    // forward packets from server to openvpn client
    let t2 = thread::spawn(move || {
        for packet in server_rx {
            println!("Got server packet: {:?}", packet);
            let mut dec = FrameDecoder::new();
            if let Some(decoded) = dec.read_frame(vec_to_frame(packet)) {
                for pkt in decoded {
                    println!("Decoded: {:?}", pkt);
                    client_tx.send(pkt).unwrap();
                }
            } else {
                println!("Failed to decode packet!");
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
