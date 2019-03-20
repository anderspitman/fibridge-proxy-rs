mod transport;
mod hoster_manager;
mod stats_conduit;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use hoster_manager::HosterManager;
use futures::Future;

type HosterManagers = Arc<Mutex<HashMap<String, HosterManager>>>;


fn main() {
    let hoster_managers = Arc::new(Mutex::new(HashMap::new()));
    let hoster_managers_clone = hoster_managers.clone();
    let range_clone = hoster_managers.clone();

    let omnis = warp::path("omnistreams")
        .map(move || hoster_managers.clone())
        .and(warp::ws2())
        .map(|hoster_managers: HosterManagers, ws: warp::ws::Ws2| {
            ws.on_upgrade(move |socket| {
                
                let hoster = HosterManager::new(socket);

                hoster_managers.lock().expect("get lock").insert(hoster.id(), hoster);

                futures::future::ok(())
            })
        });

    let ranged = warp::header::<String>("Range")
        .and(warp::path::param())
        .and(warp::path::param())
        .and_then(move |range, id: String, filename: String| {
            let mut lock = range_clone.lock().expect("get lock");
            let manager = lock.get_mut(&id).expect("get hoster manager");

            manager.process_request(filename, range)
                .map_err(|_e| warp::reject::not_found())
        });

    let non_ranged = warp::path::param()
        .and(warp::path::param())
        .and_then(move |id: String, filename: String| {

            let mut lock = hoster_managers_clone.lock().expect("get lock");
            let manager = lock.get_mut(&id).expect("get hoster manager");

            manager.process_request(filename, "".to_string())
                .map_err(|_e| warp::reject::not_found())
        });

    let download = ranged.or(non_ranged);

    let index = warp::path::end().map(|| {
        warp::reply::html("<h1>Hi there</h1>")
    });

    let routes = index
        .or(omnis)
        .or(download);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 9001));
}
