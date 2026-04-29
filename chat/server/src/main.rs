use std::io::{ErrorKind, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;   // 引入 mpsc 模块，用于线程间通信
use std::thread;       // 引入 thread 模块，用于创建线程

const LOCAL: &str = "127.0.0.1:6000";
const MSG_SIZE: usize = 32;

fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

fn main() {
    let server = TcpListener::bind(LOCAL).expect("Listener failed to bind");
    // 设置为非阻塞模式，这样 accept() 不会卡住主线程
    server.set_nonblocking(true).expect("failed to initialize non-blocking");

    // 等价于 let mut clients: Vec<Client> = vec![];
    // 或者 let mut clients = Vec::<Client>::new();
    // 宏 vec![] 能自动推导类型为 Vec<T>
    let mut clients = vec![];
    // 创建一个 mpsc 通道（channel）用于 子线程 -> 主线程 传递消息
    // mpsc = multiple producer, single consumer （多生产者，单消费者）
    // 这个通道只能传递 String 类型的数据
    // let (tx, rx) = ... 这种写法叫「解构」，同时得到发送端 tx 和接收端 rx
    let (tx, rx) = mpsc::channel::<String>();
    loop {
        if let Ok((mut socket, addr)) = server.accept() {
            println!("客户端 {} 已连接", addr);
            // 重点！！！克隆 tx 发送端，给这个新线程使用
            let tx = tx.clone();
            clients.push(socket.try_clone().expect("克隆客户端失败！"));
            // 启动一个独立的线程来处理这个客户端的读取
            thread::spawn(move || loop {
                // 创建一个 32 字节的缓冲区
                let mut buff = vec![0; MSG_SIZE];

                match socket.read_exact(&mut buff) {
                    Ok(_) => {
                            // 读取成功：把缓冲区中有效部分（直到第一个 0）转成字符串
                            let msg_bytes: Vec<u8> = buff
                                .into_iter()
                                .take_while(|&x| x != 0)   // 遇到 0 就停止（因为后面是填充的 0）
                                .collect();
                            let msg = String::from_utf8(msg_bytes)
                                .expect("接收到非 UTF-8 消息！");
                            println!("收到来自 {} 的消息: {:?}", addr, msg);
                            // 通过通道把消息发送给主线程
                            tx.send(msg).expect("通过通道发送消息失败");
                    }
                    // 非阻塞模式下，如果没有数据可读，会返回 WouldBlock 错误，我们直接忽略
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    // 其他错误（比如客户端断开连接），则结束这个线程
                    Err(_) => {
                        println!("客户端 {} 已断开连接", addr);
                        break;
                    }
                }
                sleep();
            });
        }

        // ==================== 主线程接收并广播消息 ====================
        // try_recv() 是非阻塞接收，如果通道里有消息就拿出来
        if let Ok(msg) = rx.try_recv() {
            println!("正在广播消息: {}", msg);
            // 把消息广播给所有已连接的客户端
            clients = clients
                .into_iter()
                .filter_map(|mut client| {
                    let mut buff = msg.clone().into_bytes(); // String -> Vec<u8>
                    buff.resize(MSG_SIZE, 0);                // 补齐到 32 字节，用 0 填充
                    // 尝试写入，如果成功就保留这个 client，否则过滤掉（说明已断开）
                    client.write_all(&buff).map(|_| client).ok()
                })
                .collect::<Vec<_>>();   // 重新收集剩余存活的客户端
        }
      
        sleep(); // 主循环也稍微休息一下
    }
}
