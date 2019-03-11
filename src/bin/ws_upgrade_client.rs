fn main() {
}

//use std::net::{TcpStream};
//use std::io::{Read, Write};
//use http_tester::HttpRequestBuilder;
//
//fn main() -> std::io::Result<()> {
//
//    let mut stream = TcpStream::connect("127.0.0.1:9001")?;
//
//    let request = HttpRequestBuilder::new()
//        .path("/omnistreams")
//        .header("Connection", "keep-alive, Upgrade")
//        .header("Upgrade", "websocket")
//        .build();
//
//    println!("{:?}", request.to_string());
//
//    stream.write(request.to_string().as_bytes())?;
//    //stream.write(b"GET /omnistreams HTTP/1.1\nConnection: keep-alive, Upgrade\nUpgrade: websocket\nSec-WebSocket-Key: biItRicfxX4AuzpDncBnpA==\nSec-WebSocket-Version: 13\n\n");
//
//    read_response(&mut stream)?;
//    stream.write(request.to_string().as_bytes())?;
//    read_response(&mut stream)?;
//
//    Ok(())
//}
//
//fn read_response(stream: &mut TcpStream) -> std::io::Result<()> {
//    let mut response = vec![0; 1024];
//    let read = stream.read(&mut response)?;
//    let response = &response[..read];
//    println!("{}, {:#?}", read, std::str::from_utf8(response).expect("parse response"));
//    Ok(())
//}
