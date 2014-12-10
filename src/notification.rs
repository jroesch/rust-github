extern crate serialize;
extern crate url;

use clone_url::CloneUrl;
use self::url::Url;
use self::serialize::json::Json;

// TODO: there is much more information that what is
// shown here.  This is just what is needed for Gradr.
#[deriving(Show, PartialEq)]
pub struct PushNotification {
    pub clone_url: CloneUrl,
    pub branch: String
}

pub trait ToNotification {
    fn to_push_notification(&self) -> Result<PushNotification, &'static str>;
}

impl ToNotification for Json {
    fn to_push_notification(&self) -> Result<PushNotification, &'static str> {
        let obj = try!(self.as_object().ok_or("malformed object"));
        let ref_line = 
            try!(try!(obj.get(&"ref".to_string()).ok_or("no 'ref'"))
                 .as_string().ok_or("'ref' is not a string"));
        let branch = try!(ref_line.split('/').last().ok_or("empty 'ref' line"));
        let repo_obj = 
            try!(obj.get(&"repository".to_string()).ok_or("no 'repository' object"));
        let url_string = 
            try!(try!(repo_obj.find("clone_url").ok_or("no 'clone_url'"))
                 .as_string().ok_or("'clone_url' is not a string"));
        let url = try!(Url::parse(url_string).map_err(|_| "'clone_url' is not valid"));

        match CloneUrl::new_from_url(url) {
            Some(clone_url) => {
                Ok(PushNotification {
                    clone_url: clone_url,
                    branch: branch.to_string()
                })
            },
            None => Err("URL is not a clone URL")
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate serialize;

    use self::serialize::json::Json;
    use self::serialize::json;

    use super::{ToNotification, PushNotification};
    use clone_url::CloneUrl;

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
        assert_eq!(
            valid.to_push_notification(),
            Ok(PushNotification {
                clone_url: CloneUrl::new_from_str(
                    "https://github.com/baxterthehacker/public-repo.git").unwrap(),
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
