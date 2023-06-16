use easy_ext::ext;

/// Defines a function, that accepts a list of path segments and returns a URL
macro_rules! def {
    ($vis:vis $ident:ident, $url:literal) => {
        $vis fn $ident<T: AsRef<str>>(segments: impl IntoIterator<Item = T>) -> ::url::Url {
            let mut url: ::url::Url = $url.parse().unwrap();
            url.path_segments_mut().unwrap().extend(segments);
            url
        }
    };
}

pub(crate) use def;

#[ext(UrlExt)]
pub(crate) impl url::Url {
    fn file_extension(&self) -> Option<&str> {
        self.path().rsplit('.').next()
    }
}
