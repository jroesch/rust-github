// The point of the server is to listen for notifications
// from GitHub.  Upon receiving a notification, it will
// send it to some other source and return to listening.

// For the moment, we only care about push notifications,
// and we only need to know the git url and the branch.

extern crate hyper;
extern crate serialize;

use self::hyper::{HttpResult, HttpError};
use self::hyper::server::{Server, Incoming, Handler, Listening};
use self::hyper::Ipv4Addr;
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
    listener: Listening
}

impl Drop for ConnectionCloser {
    fn drop(&mut self) {
        self.listener.close();
    }
}

impl<'a, A : NotificationReceiver + 'a> NotificationListener<'a, A> {
    pub fn new(receiver: A) -> NotificationListener<'a, A> {
        NotificationListener {
            server: Server::http(Ipv4Addr(127, 0, 0, 1), 1235),
            receiver: NotificationReceiverWrapper { wrapped: receiver }
        }
    }

    pub fn event_loop(self) -> HttpResult<ConnectionCloser> {
        self.server.listen(self.receiver).map(|listener| {
            ConnectionCloser {
                listener: listener
            }
        })
    }
}
