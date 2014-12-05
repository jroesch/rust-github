// The point of the server is to listen for notifications
// from GitHub.  Upon receiving a notification, it will
// send it to some other source and return to listening.

// For the moment, we only care about push notifications,
// and we only need to know the git url and the branch.

extern crate hyper;
extern crate serialize;

use self::hyper::HttpResult;
use self::hyper::server::{Server, Handler, Listening};
use self::hyper::server::request::Request;
use self::hyper::server::response::Response;
use self::hyper::{IpAddr, Port};
use self::hyper::net::Fresh;
use self::hyper::uri::RequestUri::AbsolutePath;
use self::hyper::method::Method::Post;
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

impl<'a> ToNotificationKind for hyper::server::request::Request<'a> {
    fn get_kind(&self) -> Option<NotificationKind> {
        match (&self.method, &self.uri) {
            (&Post, &AbsolutePath(ref path)) if path.as_slice() == "/push_hook" => {
                Some(NotificationKind::Push)
            },
            _ => None
        }
    }
}

impl<'a, A: NotificationReceiver + Sync + 'a> Handler for NotificationReceiverWrapper<'a, A> {
    #[allow(unused_must_use)]
    fn handle(&self, mut req: Request, res: Response<Fresh>) {
        let kind = req.get_kind();
        match kind {
            Some(NotificationKind::Push) => {
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

impl<'a, A : NotificationReceiver + Sync + 'a> NotificationListener<'a, A> {
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

pub mod testing {
    extern crate hyper;
    extern crate url;

    use self::hyper::{IpAddr, Port};
    use self::hyper::client::Request;
    use self::hyper::header::common::connection::Connection;
    use self::hyper::header::common::connection::ConnectionOption::Close;
    use self::url::Url;
    use self::Sendable::{SendPush, SendString};

    use notification::PushNotification;

    pub fn send_to_server(what: &str, addr: IpAddr, port: Port) {
        let mut fresh = Request::post(
            Url::parse(
                format!("http://{}:{}/push_hook",
                        addr.to_string(),
                        port).as_slice()).unwrap()).unwrap();
        fresh.headers_mut().set(Connection(vec!(Close)));
        let mut streaming = fresh.start().unwrap();
        streaming.write_str(what).unwrap();
        streaming.send().unwrap().read_to_string().unwrap();
    }

    pub enum Sendable<'a> {
        SendPush(PushNotification),
        SendString(&'a str)
    }

    impl<'a> ToString for Sendable<'a> {
        fn to_string(&self) -> String {
            match *self {
                SendPush(ref push) => {
                    format!("{{ \"ref\": \"refs/head/{}\", \"repository\": {{ \"clone_url\": \"{}\" }} }}",
                            push.branch,
                            push.clone_url.to_string())
                }
                SendString(s) => s.to_string()
            }
        }
    }
}
    
#[cfg(test)]
mod tests {
    extern crate hyper;
    extern crate url;

    use std::sync::{RWLock, Arc};
    use self::hyper::{IpAddr, Ipv4Addr, Port};
    use self::url::Url;

    use super::{NotificationReceiver, NotificationListener};

    use notification::PushNotification;
    use super::testing::{send_to_server, Sendable};
    use super::testing::Sendable::{SendPush, SendString};

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

    fn extract_pushes<'a, 'b>(from: &'a Vec<&'b Sendable>) -> Vec<&'b PushNotification> {
        let mut retval = Vec::new();
        for sendable in from.iter() {
            match sendable {
                &&SendPush(ref not) => retval.push(not),
                _ => ()
            }
        }

        retval
    }

    fn send_multi_to_server(port: Port, what: &Vec<&Sendable>) {
        let recv = TestReceiver::new();
        let mut closer =
            NotificationListener::new(ADDR, port, recv.clone()).event_loop().unwrap();

        for each in what.iter() {
            send_to_server(each.to_string().as_slice(), ADDR, port);
        }

        closer.close();

        let expected_pushes = extract_pushes(what);
        let actual_pushes = recv.pushes.read();
        
        assert_eq!(expected_pushes.len(), actual_pushes.len());

        for (a, b) in expected_pushes.iter().zip(actual_pushes.iter()) {
            assert_eq!(a, &b);
        }
    }
    
    #[test]
    fn server_gets_valid_push_1() {
        let p1 =
            &SendPush(PushNotification {
                clone_url: Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap(),
                branch: "master".to_string()
            });
        send_multi_to_server(1235u16, &vec!(p1));
    }

    #[test]
    fn server_gets_valid_push_2() {
        let p1 =
            &SendPush(PushNotification {
                clone_url: Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap(),
                branch: "master".to_string()
            });
        let p2 =
            &SendPush(PushNotification {
                clone_url: Url::parse("https://github.com/blahdeblah/coolbeans.git").unwrap(),
                branch: "experimental".to_string()
            });
        send_multi_to_server(1236u16, &vec!(p1, p2));
    }

    #[test]
    fn server_gets_invalid() {
        let s = &SendString("some string");
        send_multi_to_server(1237u16, &vec!(s));
    }

    #[test]
    fn server_gets_invalid_valid() {
        let v1 =
            &SendPush(PushNotification {
                clone_url: Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap(),
                branch: "master".to_string()
            });
        let v2 =
            &SendPush(PushNotification {
                clone_url: Url::parse("https://github.com/blahdeblah/coolbeans.git").unwrap(),
                branch: "experimental".to_string()
            });
        let s1 = &SendString("some string");
        let s2 = &SendString("some other string");

        send_multi_to_server(1238u16, &vec!(v1, s1, s2, v2));
    }
}
