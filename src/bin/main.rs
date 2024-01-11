// 版权所有 Amazon.com, Inc. 或其关联公司。保留所有权利。
// SPDX-License-Identifier: Apache-2.0


use btcmbase::datagram::CommandDataGram;
use btcmbase::datagram::MessageDataGram;
use s2n_quic::{client::Connect, Client};
use tokio::io::AsyncWriteExt;
use std::{error::Error, net::SocketAddr};

/// 注意: 该证书仅供演示目的使用！
// 静态字符串，包含 PEM 格式的证书内容
pub static CERT_PEM: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../certs/cert.pem"
));

// 异步主函数，返回 Result 类型，其中错误类型为实现 Error trait 的任意类型的 trait 对象
#[tokio::main]
// #[repr(align)]
async fn main() -> Result<(), Box<dyn Error>> {
    // 创建 Client 实例，并通过 builder 模式配置 TLS 和 I/O
    let client = Client::builder()
        .with_tls(CERT_PEM)?  // 使用指定的 PEM 证书配置 TLS
        .with_io("0.0.0.0:0")?  // 指定 I/O 地址
        .start()?;  // 启动 Client

    // 定义 Socket 地址，解析字符串并返回 SocketAddr 类型
    let addr: SocketAddr = "127.0.0.1:4433".parse()?;
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
    
    let mut data_gram_buf = MessageDataGram::create_gram_buf(1024);
    let message: &mut MessageDataGram = MessageDataGram::create_message_data_gram_by_mut_vec8(&mut data_gram_buf);
    eprintln!("Stream opened data    from {:?}", message);
    let array = data_gram_buf.as_slice();
    // eprintln!("Stream opened from {:?}", array);
    send_stream.write_all(array).await?;
    send_stream.flush().await?;

    let mut data_gram_buf = CommandDataGram::create_gram_buf();
    let command = CommandDataGram::create_command_data_gram_by_mut_u8(data_gram_buf.as_mut_slice());
    eprintln!("Stream opened data    from {:?}", command);
    let array = data_gram_buf.as_slice();
    send_stream.write_all(array).await?;
    send_stream.flush().await?;

    // 新建一个CommandDataGram   
    // 复制标准输入的数据并发送到服务器
    let mut stdin = tokio::io::stdin();
    tokio::io::copy(&mut stdin, &mut send_stream).await?;

    Ok(())  // 成功返回 Ok(())
}
