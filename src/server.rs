// The point of the server is to listen for notifications
// from GitHub.  Upon receiving a notification, it will
// send it to some other source and return to listening.

// For the moment, we only care about push notifications,
// and we only need to know the git url and the branch.

extern crate hyper;
extern crate serialize;

use self::hyper::HttpResult;
use self::hyper::server::{Server, Incoming, Handler, Listening};
use self::hyper::{IpAddr, Port};
use self::hyper::net::{HttpAcceptor, HttpStream};
use self::hyper::uri::AbsolutePath;
use self::hyper::method::Post;
use self::serialize::json::from_reader;

use notification::{ToNotification, PushNotification};

pub trait NotificationReceiver : Send {
    fn receive_push_notification(&self, not: PushNotification);
}

struct NotificationReceiverWrapper<'a, A : NotificationReceiver + 'a> {
    wrapped: A
}

enum NotificationKind {
    Push
}

trait ToNotificationKind {
    fn get_kind(&self) -> Option<NotificationKind>;
}

impl ToNotificationKind for hyper::server::request::Request {
    fn get_kind(&self) -> Option<NotificationKind> {
        match (&self.method, &self.uri) {
            (&Post, &AbsolutePath(ref path)) if path.as_slice() == "/push_hook" => {
                Some(Push)
            },
            _ => None
        }
    }
}

impl<'a, A: NotificationReceiver + 'a> Handler<HttpAcceptor, HttpStream> for NotificationReceiverWrapper<'a, A> {
    #[allow(unused_must_use)]
    fn handle(self, mut incoming: Incoming) {
        for (mut req, mut res) in incoming {
            let kind = req.get_kind();
            match kind {
                Some(Push) => {
                    match from_reader(&mut req) {
                        Ok(json) => {
                            match json.to_push_notification() {
                                Ok(not) => self.wrapped.receive_push_notification(not),
                                _ => ()
                            }
                        },
                        _ => ()
                    }
                },
                _ => ()
            };

            // needed to close the connection properly
            res.start().and_then(|res| res.end());
        }
    }
}

pub struct NotificationListener<'a, A : NotificationReceiver + 'a> {
    server: Server,
    receiver: NotificationReceiverWrapper<'a, A>
}

pub struct ConnectionCloser {
    listener: Listening,
    is_closed: bool
}

impl ConnectionCloser {
    fn new(listener: Listening) -> ConnectionCloser {
        ConnectionCloser {
            listener: listener,
            is_closed: false
        }
    }

    #[allow(unused_must_use)]
    pub fn close(&mut self) {
        self.listener.close();
        self.is_closed = true;
    }
}

impl Drop for ConnectionCloser {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        if !self.is_closed {
            self.listener.close();
        }
    }
}

impl<'a, A : NotificationReceiver + 'a> NotificationListener<'a, A> {
    pub fn new(addr: IpAddr, port: Port, receiver: A) -> NotificationListener<'a, A> {
        NotificationListener {
            server: Server::http(addr, port),
            receiver: NotificationReceiverWrapper { wrapped: receiver }
        }
    }

    pub fn event_loop(self) -> HttpResult<ConnectionCloser> {
        self.server.listen(self.receiver).map(|listener| {
            ConnectionCloser::new(listener)
        })
    }
}

#[cfg(test)]
mod tests {
    extern crate hyper;
    extern crate sync;
    extern crate url;

    use self::hyper::{IpAddr, Ipv4Addr, Port};
    use self::hyper::client::Request;
    use self::sync::{RWLock, Arc};
    use self::url::Url;

    use super::{NotificationReceiver, NotificationListener};

    use notification::PushNotification;

    static ADDR: IpAddr = Ipv4Addr(127, 0, 0, 1);

    // Cargo seems to do testing in parallel, so individual server tests
    // need different ports in order to prevent issues of addresses
    // already being in use.

    struct TestReceiver {
        pushes: RWLock<Vec<PushNotification>>
    }

    impl TestReceiver {
        fn new() -> Arc<TestReceiver> {
            Arc::new(TestReceiver { pushes: RWLock::new(Vec::new()) })
        }
    }

    impl NotificationReceiver for Arc<TestReceiver> {
        fn receive_push_notification(&self, not: PushNotification) {
            let mut lock = self.pushes.write();
            lock.push(not);
            lock.downgrade();
        }
    }

    #[test]
    fn server_starts_stops() {
        let recv = TestReceiver::new();
        NotificationListener::new(ADDR, 1234u16, recv.clone()).event_loop().unwrap().close();
        assert!(recv.pushes.read().is_empty());
    }

    fn push_notification_to_string(not: &PushNotification) -> String {
        format!("{{ \"ref\": \"refs/head/{}\", \"repository\": {{ \"clone_url\": \"{}\" }} }}",
                not.branch,
                not.clone_url.to_string())
    }

    fn send_push_notification(not: &PushNotification, port: Port) {
        let fresh = Request::post(
            Url::parse(
                format!("http://{}:{}/push_hook",
                        ADDR.to_string(),
                        port).as_slice()).unwrap()).unwrap();
        let mut streaming = fresh.start().unwrap();
        streaming.write_str(push_notification_to_string(not).as_slice()).unwrap();
        streaming.send().unwrap().read_to_string().unwrap();
    }
    
    fn server_gets_valid_pushes(port: Port, nots: Vec<PushNotification>) {
        let recv = TestReceiver::new();
        let mut closer =
            NotificationListener::new(ADDR, port, recv.clone()).event_loop().unwrap();

        for not in nots.iter() {
            send_push_notification(not, port);
        }

        closer.close();

        let pushes = recv.pushes.read();
        assert_eq!(pushes.len(), nots.len());
        
        for (a, b) in nots.iter().zip(pushes.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn server_gets_valid_push_1() {
        server_gets_valid_pushes(
            1235u16,
            vec!(
                PushNotification {
                    clone_url: Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap(),
                    branch: "master".to_string()
                }));
    }

    #[test]
    fn server_gets_valid_push_2() {
        server_gets_valid_pushes(
            1236u16,
            vec!(
                PushNotification {
                    clone_url: Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap(),
                    branch: "master".to_string()
                },
                PushNotification {
                    clone_url: Url::parse("https://github.com/blahdeblah/coolbeans.git").unwrap(),
                    branch: "experimental".to_string()
                }));
    }
}
