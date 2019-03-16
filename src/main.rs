mod transport;
mod hoster_manager;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use warp::{self, Filter};
use hoster_manager::HosterManager;

type HosterManagers = Arc<Mutex<HashMap<String, HosterManager>>>;


fn main() {
    let hoster_managers = Arc::new(Mutex::new(HashMap::new()));
    let hoster_managers_clone = hoster_managers.clone();

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

    let download = warp::path::param()
        .and(warp::path::param())
        .map(move |id: String, filename: String| {

            let mut lock = hoster_managers_clone.lock().expect("get lock");
            let manager = lock.get_mut(&id).expect("get hoster manager");

            manager.process_request(filename)
        });

    let index = warp::path::end().map(|| {
        warp::reply::html("<h1>Hi there</h1>")
    });

    let routes = index
        .or(omnis)
        .or(download);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 9001));
}
