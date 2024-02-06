// 版权所有 Amazon.com, Inc. 或其关联公司。保留所有权利。
// SPDX-License-Identifier: Apache-2.0

use btcmbase::{
    client::{ClientID, ClientPlanet, ClientType},
    datagram::{BitCommand, CommandDataGram, MessageDataGram},
};
use btcmtools::command::{self, TextToUser, UserPass};
// use bytes::Bytes;
// use btcmbase::datagram::MessageDataGram;
use s2n_quic::{client::Connect, stream::SendStream, Client};
use std::{error::Error, net::SocketAddr};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// 注意: 该证书仅供演示目的使用！
// 静态字符串，包含 PEM 格式的证书内容
pub static CERT_PEM: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../certs/cert.pem"));
static DEVICE_ID: u32 = 0x48dc7345;
// 异步主函数，返回 Result 类型，其中错误类型为实现 Error trait 的任意类型的 trait 对象
#[tokio::main]
// #[repr(align)]
async fn main() -> Result<(), Box<dyn Error>> {
    // 创建 Client 实例，并通过 builder 模式配置 TLS 和 I/O
    let client = Client::builder()
        .with_tls(CERT_PEM)? // 使用指定的 PEM 证书配置 TLS
        .with_io("0.0.0.0:0")? // 指定 I/O 地址
        .start()?; // 启动 Client

    // 定义 Socket 地址，解析字符串并返回 SocketAddr 类型
    // let addr: SocketAddr = "192.168.3.6:4433".parse()?;
    let addr: SocketAddr = "127.0.0.1:9563".parse()?;
    // let addr: SocketAddr = "117.78.10.241:9563".parse()?;
    // 创建 Connect 实例，指定服务器地址和名称
    let connect = Connect::new(addr).with_server_name("localhost");
    // 连接服务器，获取 Connection 实例
    let mut connection = client.connect(connect).await?;

    // 确保连接不会因不活动而超时
    connection.keep_alive(true)?;

    // 打开一个新的双向流，并将其拆分为接收和发送两个部分
    let stream = connection.open_bidirectional_stream().await?;
    let (mut receive_stream, mut send_stream) = stream.split();

    // 创建异步任务，将服务器的响应复制到标准输出
    tokio::spawn(async move {
        while let Ok(Some(data)) = receive_stream.receive().await {
            eprintln!("Stream opened data    from {:?}", data);
        }
        // let mut stdout = tokio::io::stdout();
        // let _ = tokio::io::copy(&mut receive_stream, &mut stdout).await;
    });

    // 新建一个CommandDataGram
    // 复制标准输入的数据并发送到服务器
    // let mut stdin = tokio::io::stdin();
    // tokio::io::copy(&mut stdin, &mut send_stream).await?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);

    // 异步读取一行字符串
    println!("Type something and press Enter:");
    let mut input = String::new();
    let mut login_userid :u32 = 0x00;
    while let Ok(_) = reader.read_line(&mut input).await {
        // println!("You typed: {}", input);
        // 021-22048310
        // let cmd = command::parse_command(&input);
        if input.ends_with('\n') {
            input.pop();
        }
        // input = input.trim_right_matches('\n').to_string();
        if let Ok((_, cmd)) = command::parse_command(&input) {
            // input.clear();
            match cmd {
                command::Command::Login(login) => {
                    eprintln!("login =  {:?}", login);
                    login_userid = login_imserver(&login,&mut send_stream).await.unwrap();
                    // continue;
                    // login.user();login.pass();
                }
                command::Command::Send(send) => {
                    eprintln!("send =  {:?}", send);
                    send_text2user(login_userid,&send,&mut send_stream).await?;
                    // continue;
                    // send.text();send.user();
                }
                command::Command::Logout(user) => {
                    eprintln!("logout =  {:?}", user);
                    logout_imserver(&user,&mut send_stream).await?;
                    // continue;
                }
                command::Command::Exit => {
                    break;
                }
                command::Command::Quit => {
                    break;
                }
            }
            // println!("可以解析: {}", input);
            // println!("You command: {}", cmd);
            input.clear();
        } else {
            input.clear();
        }
        // send_stream
        //     .send(Bytes::from(input.clone()))
        //     .await
        //     .expect("error!");
    }
    Ok(()) // 成功返回 Ok(())
}

async fn login_imserver(user :&UserPass,send_stream:&mut SendStream) -> Result<u32, Box<dyn Error>> {
    // let deviceid: u32 = 0x48dc7345;
    let login_userid  = user.user().parse().unwrap();
    let clientid: ClientID = ClientID::new(ClientPlanet::PLANET_EARTH, login_userid);
    let mut data_gram_buf = CommandDataGram::create_gram_buf(0);
    let command = CommandDataGram::create_command_data_gram_by_mut_u8(data_gram_buf.as_mut_slice());
    command.set_deviceid(DEVICE_ID); // 设置设备id
    command.set_sender(clientid); // sender -> ClientID
    command.set_sendertype(ClientType::CLIENT_PEOPLE); // Client_People
    command.set_command(BitCommand::LOGIN_COMMAND); // 登录
    eprintln!("input command {:?}", command);
    let array = data_gram_buf.as_slice();
    send_stream.write_all(array).await?;
    send_stream.flush().await?;
    Result::Ok(login_userid)
}


async fn send_text2user(loginuserid:u32,
                        text:&TextToUser,
                        send_stream:&mut SendStream) -> Result<(), Box<dyn Error>> {
    let mut message_buf = MessageDataGram::create_gram_databuf(text.text().as_bytes());
    let message_gram = MessageDataGram::create_message_data_gram_by_mut_vec8(&mut message_buf);
    message_gram.set_deviceid(DEVICE_ID); // 设置设备id
    message_gram.set_command(BitCommand::SEND_MESS);
    let sendclientid: ClientID = ClientID::new(ClientPlanet::PLANET_EARTH, loginuserid);
    message_gram.set_sender(sendclientid);
    message_gram.set_sendertype(ClientType::CLIENT_PEOPLE);
    let receuserid :u32 = text.user().parse().unwrap();
    let receclientid:ClientID = ClientID::new(ClientPlanet::PLANET_EARTH, receuserid);
    message_gram.set_receiver(receclientid);
    message_gram.set_receivertype(ClientType::CLIENT_PEOPLE);

    eprintln!("input message {:?}", message_gram);
    let array = message_buf.as_slice();
    send_stream.write_all(array).await?;
    send_stream.flush().await?;
    Result::Ok(())
}

async fn logout_imserver(user:&String,send_stream:&mut SendStream) -> Result<(), Box<dyn Error>> {
    // let deviceid: u32 = 0x48dc7345;
    let login_userid  = user.parse().unwrap();
    let mut data_gram_buf = CommandDataGram::create_gram_buf(0);
    let command = CommandDataGram::create_command_data_gram_by_mut_u8(data_gram_buf.as_mut_slice());
    let clientid: ClientID = ClientID::new(ClientPlanet::PLANET_EARTH, login_userid);
    command.set_deviceid(DEVICE_ID); // 设置设备id
    command.set_sender(clientid); // sender -> ClientID
    command.set_sendertype(ClientType::CLIENT_PEOPLE); // Client_People
    command.set_command(BitCommand::LOGOUT_COMMAND); // 登出
    eprintln!("Stream opened data    from {:?}", command);
    let array = data_gram_buf.as_slice();
    send_stream.write_all(array).await?;
    send_stream.flush().await?;
    Result::Ok(())
}