use hyper::{Method, StatusCode, Body, Request, Response, Server};
use hyper::rt::Future;
use hyper::service::{service_fn, service_fn_ok};
use hyper::header::{UPGRADE, HeaderValue};
use futures::future;
use futures::stream::Stream;
use futures::sync::mpsc::unbounded;
use omnistreams::{Acceptor, WebSocketAcceptorBuilder};


type BoxFut = Box<Future<Item=Response<Body>, Error=hyper::Error> + Send>;




fn main() {

    hyper::rt::run(hyper::rt::lazy(|| {
        
        run();
        Ok(())
    }));
}

fn run() {

    let (upgraded_tx, upgraded_rx) = unbounded();
    
    let _acceptor = WebSocketAcceptorBuilder::new()
        .port(9002)
        .upgraded_rx(upgraded_rx)
        .build();

    //let transports = acceptor.transports().expect("no transports");

    //hyper::rt::spawn(transports.for_each(|_transport| {
    //    println!("transp");
    //    //println!("{:?}", transport);
    //    Ok(())
    //})
    //.map_err(|e| {
    //    eprintln!("{:?}", e);
    //}));

    // This is our socket address...
    let addr = ([127, 0, 0, 1], 9001).into();

    let service = move || {

        let upgraded_tx = upgraded_tx.clone();

        //service_fn_ok(move |_req| {
        service_fn(move |req: Request<Body>| {
            let mut response = Response::new(Body::empty());

            let upgraded_tx = upgraded_tx.clone();

            match (req.method(), req.uri().path()) {
                (&Method::GET, "/") => {
                    *response.body_mut() = Body::from("Try POSTing data to /echo");
                },
                (&Method::GET, "/omnistreams") => {
                    let on_upgrade = req
                        .into_body()
                        .on_upgrade()
                        .map_err(|err| eprintln!("upgrade error: {}", err))
                        .and_then(move |upgraded| {
                            //upgraded.write(b"hi there");
                            //tokio::io::write_all(upgraded, b"Hi there");
                            upgraded_tx.unbounded_send(upgraded).unwrap();
                            Ok(())
                        });

                    hyper::rt::spawn(on_upgrade);

                    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
                    response.headers_mut().insert(UPGRADE, HeaderValue::from_static("websocket"));
                    //*response.body_mut() = Body::from("hit omni");
                },
                (&Method::POST, "/echo") => {
                    // we'll be back
                },
                _ => {
                    *response.status_mut() = StatusCode::NOT_FOUND;
                },
            };

            Box::new(future::ok(response)) as BoxFut
        })
    };

    let server = Server::bind(&addr)
        .serve(service)
        .map_err(|e| eprintln!("server error: {}", e));

    // Run this server for... forever!
    hyper::rt::spawn(server);
}
