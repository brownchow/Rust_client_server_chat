use std::io::{self, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

const LOCAL: &str = "127.0.0.1:6000";
const MSG_SIZE: usize = 32;

fn main() {
    // 1. 连接服务器并设置为非阻塞
    let mut client = TcpStream::connect(LOCAL).expect("服务端连接失败");
    client.set_nonblocking(true).expect("初始化非阻塞客户端失败！");

    // 2. 创建线程间通信通道
    let (tx, rx) = mpsc::channel::<String>();

    // 3. 启动子线程：负责接收和发送消息
    thread::spawn(move || {
        loop {
            // ===== 接收服务器消息 =====
            let mut buff = vec![0; MSG_SIZE];
            match client.read_exact(&mut buff) {
                Ok(_) => {
                    // 处理消息：截取到第一个 0 之前的内容
                    let msg: Vec<u8> = buff.into_iter()
                                          .take_while(|&byte| byte != 0)
                                          .collect();
                    println!("接到服务器信息: {:?}", String::from_utf8_lossy(&msg));
                },
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                Err(_) => {
                    println!("与服务器的连接已断开！");
                    break;
                }
            }

            // ===== 发送用户输入的消息 =====
            match rx.try_recv() {
                Ok(msg) => {
                    let mut buff = msg.clone().into_bytes();
                    buff.resize(MSG_SIZE, 0);           // 补齐到固定长度
                    if let Err(e) = client.write_all(&buff) {
                        println!("发送消息失败: {}", e);
                    } else {
                        println!("消息已发送: {}", msg);
                    }
                },
                Err(TryRecvError::Empty) => (),         // 没有消息，正常情况
                Err(TryRecvError::Disconnected) => break // 主线程退出，通道断开
            }

            thread::sleep(Duration::from_millis(100));
        }
    });

    // 4. 主线程：读取用户输入
    println!("输入一条消息 (输入 :quit 退出):");
    loop {
        let mut buff = String::new();
        io::stdin().read_line(&mut buff).expect("读取输入失败");

        let msg = buff.trim().to_string();

        if msg == ":quit" || tx.send(msg).is_err() {
            break;
        }
    }

    println!("客户端已退出！");
}