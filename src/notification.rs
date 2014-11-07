extern crate serialize;
extern crate url;

use self::serialize::json::Json;
use self::url::Url;

// TODO: there is much more information that what is
// shown here.
#[deriving(Show, PartialEq, Eq, Hash)]
pub struct PushNotification {
    pub clone_url: Url,
    pub branch: String
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
            try!(try!(repo_obj.find("clone_url").ok_or("no 'clone_url'"))
                 .as_string().ok_or("'clone_url' is not a string"));
        let url = try!(Url::parse(url_string).map_err(|_| "'clone_url' is not valid"));
        Ok(PushNotification {
            clone_url: url,
            branch: branch.to_string()
        })
    }
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
