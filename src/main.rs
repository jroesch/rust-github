extern crate github;
extern crate hyper;

use github::notification::PushNotification;
use github::server::{NotificationReceiver, NotificationListener};

use self::hyper::{Ipv4Addr, Port};

struct Temp;

impl NotificationReceiver for Temp {
    fn receive_push_notification(&self, not: PushNotification) {
        println!("{}", not);
    }
}

fn main() {
    NotificationListener::new(Ipv4Addr(127, 0, 0, 1), 1235, Temp).event_loop().unwrap();
    println!("SERVER READY");
}
