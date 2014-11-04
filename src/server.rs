// The point of the server is to listen for notifications
// from GitHub.  Upon receiving a notification, it will
// send it to some other source and return to listening.

// For the moment, we only care about push notifications,
// and we only need to know the git url and the branch.

extern crate hyper;
extern crate url;
extern crate serialize;

use self::hyper::server::Server;
use self::hyper::Ipv4Addr;
use self::url::Url;

use self::serialize::json::{Json, Object, JsonObject, from_reader};


// TODO: there is much more information that what is
// shown here.
#[deriving(Show, PartialEq)]
pub struct PushNotification {
    clone_url: Url,
    branch: String
}

// TODO: There are many more kinds of notifications.
pub enum Notification {
    Push(PushNotification)
}

pub trait ToNotification {
    fn to_push_notification(&self) -> Result<PushNotification, &'static str>;
}

impl ToNotification for Json {
    fn to_push_notification(&self) -> Result<PushNotification, &'static str> {
        let obj = try!(self.as_object().ok_or("malformed object"));
        let ref_line = 
            try!(try!(obj.find(&"ref".to_string()).ok_or("no 'ref'"))
                 .as_string().ok_or("'ref' is not a string"));
        let branch = try!(ref_line.split('/').last().ok_or("empty 'ref' line"));
        let repo_obj = 
            try!(obj.find(&"repository".to_string()).ok_or("no 'repository' object"));
        let url_string = 
            try!(try!(repo_obj.find(&"clone_url".to_string()).ok_or("no 'clone_url'"))
                 .as_string().ok_or("'clone_url' is not a string"));
        let url = try!(Url::parse(url_string).map_err(|_| "'clone_url' is not valid"));
        Ok(PushNotification {
            clone_url: url,
            branch: branch.to_string()
        })
    }
}

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

#[cfg(test)]
mod tests {
    extern crate serialize;
    extern crate url;
    use self::serialize::json::Json;
    use self::serialize::json;
    use self::url::Url;

    use super::{ToNotification, PushNotification};

    fn as_json(s: &str) -> Json {
      json::from_str(s).unwrap()
    }

    #[test]
    fn json_sanity_check() {
        let test = "{\"ref\": \"refs/heads/gh-pages\", \"repository\":\
            { \"clone_url\": \"https://github.com/baxterthehacker/public-repo.git\" } }";
        as_json(test);
    }

    #[test]
    fn parse_valid_push() {
        let valid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\", \"repository\":\
                    { \"clone_url\": \
                    \"https://github.com/baxterthehacker/public-repo.git\" } }");
        let url = Url::parse("https://github.com/baxterthehacker/public-repo.git").unwrap();
        assert_eq!(
            valid.to_push_notification(),
            Ok(PushNotification {
                clone_url: url,
                branch: "gh-pages".to_string()
            }));
    }

    #[test]
    fn parse_invalid_push_missing_ref() {
        let missing_ref = 
            as_json("{\"repository\":\
                    { \"clone_url\": \
                    \"https://github.com/baxterthehacker/public-repo.git\" } }");
        assert!(missing_ref.to_push_notification().is_err());
    }

    #[test]
    fn parse_invalid_push_ref_nonstring() {
        let invalid = 
            as_json("{\"ref\": 5, \"repository\":\
                    { \"clone_url\": \
                    \"https://github.com/baxterthehacker/public-repo.git\" } }");
        assert!(invalid.to_push_notification().is_err());
    }

    #[test]
    fn parse_invalid_push_missing_repository() {
        let invalid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\" }");
        assert!(invalid.to_push_notification().is_err());
    }

    #[test]
    fn parse_invalid_push_repository_nonobject() {
        let invalid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\", \"repository\": 5 }");
        assert!(invalid.to_push_notification().is_err());
    }

    #[test]
    fn parse_invalid_push_missing_clone_url() {
        let invalid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\", \"repository\": { } }");
        assert!(invalid.to_push_notification().is_err());
    }
    
    #[test]
    fn parse_invalid_push_clone_url_nonstring() {
        let invalid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\", \"repository\":\
                    { \"clone_url\": 5 } }");
        assert!(invalid.to_push_notification().is_err());
    }

    #[test]
    fn parse_invalid_push_clone_url_nonurl() {
        let invalid = 
            as_json("{\"ref\": \"refs/heads/gh-pages\", \"repository\":\
                    { \"clone_url\": \"blahbaz\" } }");
        assert!(invalid.to_push_notification().is_err());
    }
}
    