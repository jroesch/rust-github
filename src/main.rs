extern crate github;

use github::notification::PushNotification;
use github::server::{NotificationReceiver, NotificationListener};

struct Temp;

impl NotificationReceiver for Temp {
    fn receive_push_notification(&self, not: PushNotification) {
        println!("{}", not);
    }
}

fn main() {
    NotificationListener::new(Temp).event_loop().unwrap();
    println!("SERVER READY");
}
