// The point of the server is to listen for notifications
// from GitHub.  Upon receiving a notification, it will
// send it to some other source and return to listening.

// For the moment, we only care about push notifications,
// and we only need to know the git url and the branch.

extern crate hyper;

use self::hyper::server::Server;
use self::hyper::Ipv4Addr;

use notification::Notification;

trait NotificationReceiver {
    fn receive_notification(not: Notification);
}

// impl Handler<HttpAcceptor, HttpStream> for NotificationReceiver {
//     fn handle(self, mut incoming: Incoming) {
//         for (mut req, mut res) in incoming {
//             match req.uri {
//                 hyper::uri::AbsolutePath(ref path) => {
//                     match (&req.method, path.as_slice()) {
//                         (&Post, "push_hook") => {
//                             from_reader(&mut req, 
struct NotificationListener<'a, A : NotificationReceiver + 'a> {
    server: Server,
    receiver: A
}

impl<'a, A : NotificationReceiver + 'a> NotificationListener<'a, A> {
    fn new(receiver: A) -> NotificationListener<'a, A> {
        NotificationListener {
            server: Server::http(Ipv4Addr(127, 0, 0, 1), 1235),
            receiver: receiver
        }
    }

//     fn event_loop(self) {
//         self.listen(
}
