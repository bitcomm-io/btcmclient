// 版权所有 Amazon.com, Inc. 或其关联公司。保留所有权利。
// SPDX-License-Identifier: Apache-2.0

use btcmbase::{
    client::{ClientID, ClientPlanet, ClientType},
    datagram::{BitCommand, CommandDataGram, MessageDataGram},
};
use btcmtools::command::{self, TextToUser, UserPass};
use bytes::Bytes;
// use bytes::Bytes;
// use btcmbase::datagram::MessageDataGram;
use s2n_quic::{client::Connect, stream::{ReceiveStream, SendStream}, Client, Connection};
use std::{error::Error, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, sync::Mutex};

/// 注意: 该证书仅供演示目的使用！
// 静态字符串，包含 PEM 格式的证书内容
pub static CERT_PEM: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../certs/cert.pem"));
static DEVICE_ID: u32 = 0x48dc7346;


async fn get_quic_connect() -> Result<Option<Connection>,Box<dyn Error>> {
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

    Result::Ok(Some(connection))
}
async fn rece_data(stream:Arc<Mutex<ReceiveStream>>) {
    let mut receive_stream = stream.lock().await;
    while let Ok(Some(data)) = receive_stream.receive().await {
        eprintln!("read from server {:?}", data);
    }
}

async fn inputext(stream:Arc<Mutex<SendStream>>) -> Result<Option<()>,Box<dyn Error>> {

    let mut send_stream = stream.lock().await;
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
        } else {
        send_stream
            .send(Bytes::from(input.clone()))
            .await?;
        }
        input.clear();
    }    
    Ok(Option::None)
}
static _TIME_OUT_:Duration = Duration::from_secs(5);
// 异步主函数，返回 Result 类型，其中错误类型为实现 Error trait 的任意类型的 trait 对象
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //
    while 1==1 {
        // 
        let mut connection = get_quic_connect().await.unwrap().unwrap();//client.connect(connect).await?;

        // 打开一个新的双向流，并将其拆分为接收和发送两个部分
        let stream = connection.open_bidirectional_stream().await?;
        //
        let (receive_stream, send_stream) = stream.split();
        //
        let rs = Arc::new(Mutex::new(receive_stream));
        //
        tokio::spawn(async move {rece_data(rs).await;});
        //
        let ss = Arc::new(Mutex::new(send_stream));
        // 
        let res = inputext(ss).await;
        match res {
            Ok(_) => {
                break;
            }
            Err(err) => {
                eprintln!("quic error =  {:?}", err);
            }
        }
        eprintln!("client re connection! ");
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