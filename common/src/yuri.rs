/// Yes URIs! Simple macro for making relative URLs from a bunch of given path
/// segments. The only special character is '/', which when encountered will
/// cause the given string to be treated as two separate segments.
///
/// Example:
/// ```rust
/// use pigweb_common::{yuri, query};
///
/// fn main() {
///     let update = yuri!("api", "pigs", "update");
///     let create = yuri!("/api/pigs", "create");
///     let fetch = yuri!("api/pigs", "fetch" ;? query!("name" = "Obamna"));
///
///     assert_eq!("/api/pigs/update", update);
///     assert_eq!("/api/pigs/create", create);
///     assert_eq!("/api/pigs/fetch?name=Obamna", fetch);
/// }
#[macro_export]
macro_rules! yuri {
    ($($segment:expr),+) => {{
        let mut res = String::new();

        $(
            // For each token we have, split if there are any slashes present
            let split: Vec<&str> = $segment.split('/').collect();

            // Then add each part as a path segment to the main uri
            for part in split {
                if !part.is_empty() {
                    res.push('/');
                    // push_str expects &str, and without ::<String> collect() doesn't know what to return
                    res.push_str(form_urlencoded::byte_serialize(part.as_bytes()).collect::<String>().as_str());
                }
            }
        )+

        res
    }};
    ($($segment:expr),+ ;? $query:expr) => {{
        // Compile the path segments into a string and compute the query string
        let segment = yuri!($($segment),+);
        let query = $query;

        // If we have a query param, append it to the segment
        if !query.is_empty() {
            format!("{}?{}", segment, query)
        } else {
            segment
        }
    }};
}

/// Quickly and easily constructs a URL query parameter string. Accepts either a
/// struct which derives [serde::Serialize] or a mapping of key/value pairs.
///
/// Example:
/// ```rust
/// use serde::Serialize;
/// use pigweb_common::query;
///
/// #[derive(Serialize)]
/// struct SearchQuery {
///     pub name: String,
///     pub id: u16
/// }
///
/// fn main() {
///     let create_query = query!("name" = "Grover Cleveland", "id" = "22");
///     let search_query = query!(SearchQuery { name: "Grover Cleveland".to_owned(), id: 22 });
///     let expected = "name=Grover+Cleveland&id=22";
///
///     assert_eq!(expected, create_query);
///     assert_eq!(expected, search_query);
/// }
/// ```
#[macro_export]
macro_rules! query {
    ($($key:literal = $val:expr),+) => {{
        let mut res = form_urlencoded::Serializer::new(String::new());

        // For each pair, append it to the serializer
        $(res.append_pair($key, $val);)+

        res.finish()
    }};
    // The other one needs to match first
    ($serializable:expr) => {{
        // Serialize struct to query url
        serde_url_params::to_string(&$serializable).ok().unwrap_or_default()
    }};
}
