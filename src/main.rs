mod transport;
mod hoster_manager;
mod stats_conduit;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use warp::http::{Response};
use hoster_manager::HosterManager;
use futures::Future;
use futures::future::Either;
use clap::{App, Arg};
use std::net::SocketAddrV4;
use rand::Rng;

type HosterManagers = Arc<Mutex<HashMap<String, HosterManager>>>;


fn main() {
    let matches = App::new("fibridge proxy")
        .about("Share local files via HTTP streaming")
        .arg(Arg::with_name("ip")
             .short("i")
             .long("ip-address")
             .value_name("IP")
             .takes_value(true))
        .arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .value_name("PORT")
             .takes_value(true))
        .get_matches();

    let port = matches.value_of("port").unwrap_or("9001");
    let ip = matches.value_of("ip").unwrap_or("127.0.0.1");
    let addr = format!("{}:{}", ip, port);

    let hoster_managers = Arc::new(Mutex::new(HashMap::new()));
    let hoster_managers_clone = hoster_managers.clone();
    let range_clone = hoster_managers.clone();

    let omnis = warp::path("omnistreams")
        .map(move || hoster_managers.clone())
        .and(warp::ws2())
        .map(|hoster_managers: HosterManagers, ws: warp::ws::Ws2| {
            ws.on_upgrade(move |socket| {
                
                let mut id = generate_id();

                // ensure id is unique
                while hoster_managers.lock().expect("get lock").get(&id).is_some() {
                    println!("not unique");
                    id = generate_id();
                }

                dbg!(hoster_managers.lock().expect("get lock").keys());

                let hoster = HosterManager::new(id, socket);

                hoster_managers.lock().expect("get lock").insert(hoster.id(), hoster);

                futures::future::ok(())
            })
        });

    // TODO: reduce duplication with non_ranged below
    let ranged = warp::header::<String>("Range")
        .and(warp::path::param())
        .and(warp::path::param())
        .and_then(move |range, id: String, filename: String| {
            let mut lock = range_clone.lock().expect("get lock");

            println!("GET /{}/{} {}", id, filename, range);

            match lock.get_mut(&id) {
                Some(manager) => {
                    Either::A(manager.process_request(filename, range)
                        .map_err(|_e| warp::reject::not_found()))
                },
                None => {
                    // TODO: This still feels super hacky. There's got to be some way to have these
                    // all be part of the same future.
                    Either::B(futures::future::ok(Response::builder()
                            .status(404)
                            .body("Not found".into())
                            .expect("error response")))
                },
            }
        });

    let non_ranged = warp::path::param()
        .and(warp::path::param())
        .and_then(move |id: String, filename: String| {

            println!("GET /{}/{}", id, filename);

            let mut lock = hoster_managers_clone.lock().expect("get lock");

            match lock.get_mut(&id) {
                Some(manager) => {
                    Either::A(manager.process_request(filename, "".to_string())
                        .map_err(|_e| warp::reject::not_found()))
                },
                None => {
                    // TODO: This still feels super hacky. There's got to be some way to have these
                    // all be part of the same future.
                    Either::B(futures::future::ok(Response::builder()
                            .status(404)
                            .body("Not found".into())
                            .expect("error response")))
                },
            }
        });

    let download = warp::get2().and(ranged.or(non_ranged));

    let index = warp::path::end().map(|| {
        warp::reply::html(include_str!("../webgui/dist/index.html"))
    });

    let routes = index
        .or(omnis)
        .or(download);

    warp::serve(routes)
        .run(addr.parse::<SocketAddrV4>().expect("parse address"));
}

fn generate_id() -> String {
    let mut rng = rand::thread_rng();
    let possible = "0123456789abcdefghijkmnpqrstuvwxyz";

    let mut random_char = || {
        let rand_index = rng.gen_range(0, possible.len());
        possible.chars().nth(rand_index).expect("error generating index")
    };

    let mut id = String::new();

    for _ in 0..4 {
        id.push(random_char());
    }

    id.push('-');

    for _ in 0..4 {
        id.push(random_char());
    }

    id
}
